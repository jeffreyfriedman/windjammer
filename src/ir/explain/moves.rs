//! Uses after a move, predicted from the emitted Rust: where rustc would report E0382 for a program the
//! interpreter runs (it copies values, so it never sees the move). Read from the generated code, the
//! prediction shares every decision the compiler made (which receivers and formals it takes by value,
//! where it inserted clones, which types are `Copy`) instead of re-deriving them (ADR-0030).
//!
//! Classes:
//! - a. `v.m()` where the emitted `Type::m` takes `self` by value;
//! - b. `f(v)` / `x.m(v)` where the callee's emitted formal is owned (a bare `v`, no clone);
//! - b'. `let w = v`;
//! - d. `let w = v.f` (a partial move), then `v` used whole.
//!
//! Each is reported only when `v` is read again before anything reassigns it. A move inside a loop of a
//! value declared outside it counts as read again (class c, the next iteration). Precision first: a value
//! whose type the emitted code does not spell, or which might be `Copy`, is not reported.

use super::calls::find_callee;
use super::emitted::{ArgForm, EmittedFile, EmittedFn, EmittedMode};

#[derive(Debug, Clone, PartialEq)]
pub enum MoveKind {
    /// Class a: the callee (`Type::method`) takes `self` by value.
    Receiver { method: String },
    /// Class b: passed to an owned formal of `callee`.
    Argument { callee: String },
    /// Class b': bound to `to` by `let to = var`.
    Let { to: String },
    /// Class d: the field moved out by `let to = var.field`.
    Field { field: String, to: String },
}

/// A move of `var` in `function` that is followed by a use rustc rejects.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    /// The function containing the move (`name` or `Type::name`).
    pub function: String,
    pub var: String,
    pub kind: MoveKind,
    /// The move is inside a loop that does not declare `var` (the use is the next iteration's).
    pub in_loop: bool,
}

pub fn moves(file: &EmittedFile) -> Vec<Move> {
    let mut out = Vec::new();
    let mut names: Vec<&String> = file.fns.keys().collect();
    names.sort();
    for name in names {
        let f = &file.fns[name];
        let mut partial = Vec::new();
        let mut push = |var: &str, seq: usize, depth: usize, kind: MoveKind| {
            if let Some(in_loop) = read_after_move(f, var, seq, depth) {
                out.push(Move { function: name.clone(), var: var.to_string(), kind, in_loop });
            }
        };
        for call in &f.calls {
            // a method is the receiver type's own (`HashMap::get` is not a user `get`): resolved by the type of a
            // bare receiver the emitted code spells; anything else is not resolved
            let callee = if call.is_method {
                let recv_ty = call.receiver.as_ref().and_then(|r| r.root.as_deref()).and_then(|v| match v {
                    "self" => name.split_once("::").map(|(t, _)| t.to_string()),
                    v => var_type(f, v),
                });
                recv_ty.and_then(|t| file.fns.get(&format!("{t}::{}", call.callee)))
            } else {
                find_callee(file, &call.callee, false)
            };
            // a. by-value receiver
            if let (Some(recv), Some(callee)) = (&call.receiver, callee) {
                if let Some(var) = &recv.root {
                    let by_value = callee.params.first().is_some_and(|p| p.name == "self" && p.mode == EmittedMode::Owned);
                    let ty = callee.name.rsplit_once("::").map(|(t, _)| t).unwrap_or("");
                    if by_value && owned_local(f, var) && !file.copy_types.contains(ty) {
                        push(var, recv.seq, recv.loop_depth, MoveKind::Receiver { method: callee.name.clone() });
                    }
                }
            }
            // b. a bare variable into an owned formal whose type is not Copy
            let Some(callee) = callee else { continue };
            let skip_self = call.is_method && callee.params.first().is_some_and(|p| p.name == "self");
            for (a, formal) in call.args.iter().zip(callee.params.iter().skip(usize::from(skip_self))) {
                let Some(var) = &a.root else { continue };
                if a.form == ArgForm::Bare && formal.mode == EmittedMode::Owned && owned_local(f, var) && moves_type(file, &formal.ty) {
                    push(var, a.seq, a.loop_depth, MoveKind::Argument { callee: callee.name.clone() });
                }
            }
        }
        for l in &f.lets {
            // b'. `let to = var` of a value whose type is spelled and not Copy
            for (var, seq, depth) in &l.moved_from {
                if owned_local(f, var) && var_type(f, var).is_some_and(|t| moves_type(file, &t)) {
                    push(var, *seq, *depth, MoveKind::Let { to: l.name.clone() });
                }
            }
            // d. `let to = var.field` of a field whose type is not Copy, then `var` used whole
            if let Some((var, field, seq, depth)) = &l.field_from {
                let fty = var_type(f, var).and_then(|t| file.struct_fields.get(&t)).and_then(|fs| fs.get(field)).cloned();
                if owned_local(f, var) && fty.is_some_and(|t| moves_type(file, &t)) {
                    if let Some(in_loop) = whole_read_after(f, var, *seq, *depth) {
                        partial.push(Move {
                            function: name.clone(),
                            var: var.clone(),
                            kind: MoveKind::Field { field: field.clone(), to: l.name.clone() },
                            in_loop,
                        });
                    }
                }
            }
        }
        out.extend(partial);
    }
    out
}

/// `var` is a local or an owned parameter of `f` (declared in it), not `self` (a reference parameter is
/// reborrowed).
fn owned_local(f: &EmittedFn, var: &str) -> bool {
    var != "self" && f.decl_depth.contains_key(var) && f.params.iter().find(|p| p.name == var).map_or(true, |p| p.mode == EmittedMode::Owned)
}

/// The type of `var` in `f` when the emitted code spells it (a parameter's, a `let`'s), without generics.
fn var_type(f: &EmittedFn, var: &str) -> Option<String> {
    let t = match f.params.iter().find(|p| p.name == var) {
        Some(p) => Some(p.ty.replace(' ', "")),
        None => f.lets.iter().rev().find(|l| l.name == var).and_then(|l| l.ty.clone()),
    }?;
    Some(t.split('<').next().unwrap_or(&t).to_string())
}

/// A value of the emitted type `ty` moves (is not `Copy`): only types known to move are reported. Generic
/// parameters, closures and anything not recognized count as possibly `Copy`.
fn moves_type(file: &EmittedFile, ty: &str) -> bool {
    let t = ty.replace(' ', "");
    let head = t.split('<').next().unwrap_or(&t);
    match head {
        "String" | "Vec" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | "VecDeque" | "Box" | "Rc" | "Arc" => true,
        "Option" => {
            let inner = t.strip_prefix("Option<").and_then(|r| r.strip_suffix('>')).unwrap_or("");
            moves_type(file, inner)
        }
        _ => file.struct_fields.contains_key(head) && !file.copy_types.contains(head),
    }
}

/// `var` is read after the move at `seq` before anything assigns it again: `Some(in_loop)`. A later
/// reassignment of `var` anywhere clears it (it may re-arm the value for a loop's next iteration).
fn read_after_move(f: &EmittedFn, var: &str, seq: usize, depth: usize) -> Option<bool> {
    read_after(f, var, seq, depth, |_| true)
}

/// The shared rule: a use (`counts`) after `seq` and before the next reassignment or new binding of the name;
/// with none, the next iteration of a loop that does not declare it, unless a `return`/`break` follows.
fn read_after(f: &EmittedFn, var: &str, seq: usize, depth: usize, counts: impl Fn(usize) -> bool) -> Option<bool> {
    let rebound = f.assigns.iter().chain(f.bindings.iter()).filter(|(s, n)| n == var && *s > seq).map(|(s, _)| *s).min();
    let limit = rebound.unwrap_or(usize::MAX);
    // a later position on another arm of an `if`/`match` never runs after the move, and one behind a
    // `return`/`break`/`continue` may be on another path: neither counts
    let reached = |u: usize| !exclusive(f, seq, u) && !f.exits.iter().any(|e| *e >= seq && *e < u);
    if f.uses.iter().any(|(s, n, _)| n == var && *s > seq && *s < limit && counts(*s) && reached(*s)) {
        return Some(false);
    }
    // the binding in force at the move: the last one before it (a parameter's is depth 0)
    let decl = f
        .bindings
        .iter()
        .filter(|(s, n)| n == var && *s < seq)
        .map(|(s, _)| *s)
        .max()
        .map_or(0, |s| f.binding_depth.get(&s).copied().unwrap_or(0));
    let in_loop = depth > decl;
    (in_loop && rebound.is_none() && !f.exits.iter().any(|e| *e >= seq)).then_some(true)
}

/// Class d: `var` used whole (not as the base of a field access) after the partial move at `seq`, or the
/// move is inside a loop that does not declare `var`.
fn whole_read_after(f: &EmittedFn, var: &str, seq: usize, depth: usize) -> Option<bool> {
    read_after(f, var, seq, depth, |s| !f.field_bases.contains(&s))
}

/// Positions `a` and `b` lie on different arms of one `if`/`match`.
fn exclusive(f: &EmittedFn, a: usize, b: usize) -> bool {
    let (Some(pa), Some(pb)) = (path_at(f, a), path_at(f, b)) else { return false };
    pa.iter().zip(pb).any(|(x, y)| x.0 == y.0 && x.1 != y.1)
}

/// The branch path at position `s`: the path of the nearest recorded position at or before it (a move's
/// position is the last use inside its own expression).
fn path_at(f: &EmittedFn, s: usize) -> Option<&Vec<(usize, usize)>> {
    (0..=s).rev().find_map(|k| f.paths.get(&k))
}
