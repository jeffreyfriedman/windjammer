//! Maps in machine code: `match m.get(&k) { Some(v) => .., None => .. }` and `m.insert(k, x)` on a `HashMap` local
//! with integer keys (`rt::map_get`, `rt::map_insert`, the lookups `mapfast.rs` does in the closure engine).
//!
//! The value's type is speculated from the map at entry (`Class::Map`); a value of another type fails the
//! helper, and the closure engine runs the statement. An arm runs at most one statement (an assignment, an
//! effect, or an `if` of such), so a failed check inside an arm has changed nothing yet: the exit resumes at
//! the whole `match` (or the `let` it initialises), which the closure engine then runs from its start.

use super::kinds::{code, Class, Elem, Ty};
use super::lower::{cty, Bind, Lo, Val, E, NO_SLOT, S};
use super::rt;
use crate::interpreter::int::IntTy;
use crate::parser::{EnumPatternBinding as P, Expression, MatchArm, Pattern, Statement, UnaryOp};
use cranelift_codegen::ir::{types, InstBuilder, MemFlags};

/// What an arm of the `match` takes.
enum Arm {
    /// `Some(name)` / `Some(_)`.
    Some(Option<&'static str>),
    /// `None`.
    None,
    /// `_`.
    Any,
}

fn arm_kind(arm: &'static MatchArm<'static>) -> Option<Arm> {
    if arm.guard.is_some() {
        return None;
    }
    Some(match &arm.pattern {
        Pattern::Wildcard => Arm::Any,
        Pattern::EnumVariant(n, P::Single(x)) if n == "Some" || n == "Option::Some" => Arm::Some(Some(x.as_str())),
        Pattern::EnumVariant(n, P::Wildcard) if n == "Some" || n == "Option::Some" => Arm::Some(None),
        Pattern::EnumVariant(n, P::None) if n == "None" || n == "Option::None" => Arm::None,
        Pattern::Identifier(n) if n == "None" => Arm::None,
        _ => return None,
    })
}

impl Lo<'_, '_> {
    /// The map local `m.get(k)` / `m.get(&k)` reads: its slot, the class's value type, and the key expression.
    fn map_lookup(&self, value: E) -> Option<(usize, Elem, E)> {
        if self.it.fast_off("mapjit") {
            return None;
        }
        let Expression::MethodCall { object: Expression::Identifier { name, .. }, method, type_args: None, arguments, .. } = value else { return None };
        let [(None, key)] = arguments.as_slice() else { return None };
        if method != "get" {
            return None;
        }
        let Bind::Handle(slot, Class::Map(elem)) = self.lookup(name)?.clone() else { return None };
        let key = match key {
            Expression::Unary { op: UnaryOp::Ref, operand, .. } => *operand,
            k => *k,
        };
        Some((slot, elem, key))
    }

    /// A `match` over `m.get(&k)` with `Some`/`None`/`_` arms. `want_value`: the `match` is an expression (its
    /// arms are expressions; `Some(Some(v))` is its value); otherwise a statement (`Some(None)`).
    pub(super) fn map_match(&mut self, value: E, arms: &'static [MatchArm<'static>], want_value: bool) -> Option<Option<Val>> {
        let (slot, elem, key) = self.map_lookup(value)?;
        let kinds: Vec<Arm> = arms.iter().map(arm_kind).collect::<Option<_>>()?;
        // the arm taken when the key is present, and when it is absent (the first arm that matches, as the walker)
        let some_at = kinds.iter().position(|k| matches!(k, Arm::Some(_) | Arm::Any))?;
        let none_at = kinds.iter().position(|k| matches!(k, Arm::None | Arm::Any))?;
        let vt = match elem {
            Elem::Scalar(t) => t,
            // an empty map: the value is what the other arm yields (a literal adopts it), else refused
            _ if want_value => Ty::Int(IntTy::Lit),
            _ => return None,
        };
        let k = self.expr(key)?;
        let Ty::Int(_) = k.ty else { return None };
        let (s, kt, w) = (self.k(slot as i64), self.k(code(k.ty)), self.k(code(vt)));
        // the call's failure exit is the statement's, made before any arm runs (arms add no exits of their own)
        let bits = self.call("map_get", rt::map_get as *const () as usize, &[s, k.v, kt, w], false, true);
        let found = self.b.ins().load(types::I64, MemFlags::trusted(), self.ctx, rt::FOUND);
        let (sb, nb, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
        self.b.ins().brif(found, sb, &[], nb, &[]);
        // a refusal from here on leaves blocks open: the statement cannot fall back to its closure code
        self.open_joins += 1;
        let tys0 = self.tys();
        // the key present
        self.b.switch_to_block(sb);
        self.b.seal_block(sb);
        self.push_scope();
        if let Arm::Some(Some(name)) = kinds[some_at] {
            let id = self.new_var(NO_SLOT, vt, true);
            let x = self.from_bits(bits, vt);
            self.b.def_var(self.vars[id].var, x);
            self.bind(name, Bind::Var(id));
        }
        let some_v = self.arm_body(arms[some_at].body, want_value);
        self.pop_scope();
        let some_v = some_v?;
        let res = some_v.as_ref().map(|a| {
            let var = self.b.declare_var(cty(a.ty));
            self.b.def_var(var, a.v);
            var
        });
        self.b.ins().jump(join, &[]);
        let tys_s = self.tys();
        // the key absent
        self.set_tys(&tys0);
        self.b.switch_to_block(nb);
        self.b.seal_block(nb);
        let none_v = self.arm_body(arms[none_at].body, want_value)?;
        let t = match (&some_v, &none_v, res) {
            (Some(a), Some(b), Some(var)) => {
                let t = match (a.ty, b.ty) {
                    (x, y) if x == y => x,
                    (Ty::Int(t), Ty::Int(IntTy::Lit)) if b.k.is_some() => Ty::Int(t),
                    (Ty::Int(IntTy::Lit), Ty::Int(t)) if a.k.is_some() => Ty::Int(t),
                    _ => return None,
                };
                self.b.def_var(var, b.v);
                Some((t, var))
            }
            (None, None, None) => None,
            _ => return None,
        };
        self.b.ins().jump(join, &[]);
        if tys_s != self.tys() {
            self.refused(|| "a local's width differs between the arms of a map lookup".into());
            return None;
        }
        self.b.switch_to_block(join);
        self.b.seal_block(join);
        self.open_joins -= 1;
        self.it.fast.mapjit += 1;
        Some(t.map(|(t, var)| Val::of(self.b.use_var(var), t)))
    }

    /// An arm's body: its value (`want_value`), or the at most one statement it runs.
    fn arm_body(&mut self, body: E, want_value: bool) -> Option<Option<Val>> {
        let r = if want_value {
            match body {
                Expression::Block { .. } => return None,
                e => Some(self.expr(e)?),
            }
        } else {
            match body {
                Expression::Block { statements, .. } => match statements.as_slice() {
                    [] => None,
                    [s] => {
                        self.arm_stmt(s)?;
                        None
                    }
                    _ => return None,
                },
                _ => return None,
            }
        };
        Some(r)
    }

    /// One statement of an arm: an assignment, an effect, or an `if` whose branches are such statements.
    fn arm_stmt(&mut self, s: S) -> Option<()> {
        match s {
            Statement::Assignment { target, value, compound_op, .. } => self.assign(target, value, *compound_op),
            Statement::Expression { expr, .. } => self.effect(expr),
            Statement::If { condition, then_block, else_block, .. } => {
                let one = |l: &'static [S]| -> Option<Option<S>> {
                    match l {
                        [] => Some(None),
                        [s] => Some(Some(*s)),
                        _ => None,
                    }
                };
                let t = one(then_block)?;
                let e = match else_block {
                    Some(l) => one(l)?,
                    None => None,
                };
                let c = self.cond(condition)?;
                let (tb, eb, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(c, tb, &[], eb, &[]);
                let tys0 = self.tys();
                self.b.switch_to_block(tb);
                self.b.seal_block(tb);
                if let Some(s) = t {
                    self.arm_stmt(s)?;
                }
                self.b.ins().jump(join, &[]);
                let tys_t = self.tys();
                self.set_tys(&tys0);
                self.b.switch_to_block(eb);
                self.b.seal_block(eb);
                if let Some(s) = e {
                    self.arm_stmt(s)?;
                }
                self.b.ins().jump(join, &[]);
                if tys_t != self.tys() {
                    return None;
                }
                self.b.switch_to_block(join);
                self.b.seal_block(join);
                Some(())
            }
            _ => None,
        }
    }

    /// `m.insert(k, x)` on a map local with an integer key and a scalar value. `None`: not such a call.
    pub(super) fn map_insert(&mut self, e: E) -> Option<Option<()>> {
        let Expression::MethodCall { object: Expression::Identifier { name, .. }, method, type_args: None, arguments, .. } = e else { return None };
        if method != "insert" || self.it.fast_off("mapjit") {
            return None;
        }
        let Bind::Handle(slot, Class::Map(_)) = self.lookup(name)?.clone() else { return None };
        let [(None, key), (None, x)] = arguments.as_slice() else { return Some(None) };
        Some((|| {
            let k = self.expr(key)?;
            let Ty::Int(_) = k.ty else { return None };
            let x = self.expr(x)?;
            let bits = self.to_bits(x.v, x.ty);
            let (s, kt, xt) = (self.k(slot as i64), self.k(code(k.ty)), self.k(code(x.ty)));
            self.call("map_insert", rt::map_insert as *const () as usize, &[s, k.v, kt, bits, xt], false, true);
            self.it.fast.mapjit += 1;
            Some(())
        })())
    }
}
