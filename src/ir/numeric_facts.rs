//! Publication of the numeric solver's widths as facts on `IrFunction` (ADR-0028).
//!
//! Facts: NUM-LITERAL-WIDTH (`literal_widths`), NUM-LOCAL-I64 (`solver_i64_locals`),
//! NUM-LOCAL-WIDTH (`solver_local_widths`) and NUM-USIZE (`index_source_widths`). A width is
//! published only when the solver fixed it (never the `int` default) and the class it belongs to
//! needs nothing the backend cannot print yet: classes in which an index source or use resolved to
//! another width (they need a NUM-USIZE-TO-INT conversion) and classes that contain a local the
//! facts cannot name (shadowed or closure-bound) stay with the legacy deciders.

use crate::ir::constraint_gen::FunctionConstraints;
use crate::ir::constraints::ConstraintVar;
use crate::ir::node::IrFunction;
use crate::ir::safety_type::BaseType;
use crate::ir::solver::{is_integer_type, SolverResult};
use std::collections::BTreeSet;

/// Where a NUM-USIZE-TO-INT conversion goes: the expression's source position and its kind (a
/// binary expression and the token after it can share a position).
pub type ConversionKey = (usize, usize, u8);

/// Key of an expression that may carry a conversion (`None` without a position).
pub fn conversion_key(e: &crate::parser::Expression<'_>) -> Option<ConversionKey> {
    use crate::parser::Expression as E;
    let kind = match e {
        E::Identifier { .. } => 1,
        E::Binary { .. } => 2,
        E::MethodCall { .. } => 3,
        E::Call { .. } => 4,
        E::Index { .. } => 5,
        E::FieldAccess { .. } => 6,
        _ => 0,
    };
    let l = e.location()?;
    Some((l.line, l.column, kind))
}

struct View<'a> {
    result: &'a SolverResult,
    held: BTreeSet<u32>,
}

impl View<'_> {
    fn class(&self, v: ConstraintVar) -> Option<u32> {
        self.result.class_of.get(v.0 as usize).copied()
    }

    fn int_type(&self, v: ConstraintVar) -> Option<BaseType> {
        self.result.types.get(v.0 as usize).cloned().flatten().filter(is_integer_type)
    }

    /// The solver's integer width for this variable (a width some use fixed, or the `int` default) when its class is publishable.
    fn published(&self, v: ConstraintVar) -> Option<BaseType> {
        if self.class(v).is_none_or(|c| self.held.contains(&c)) {
            return None;
        }
        self.int_type(v)
    }

    /// The solver's float width for this literal (from an `f32` field, parameter or operand, else the `float`
    /// default) when its class is publishable.
    fn published_float(&self, v: ConstraintVar) -> Option<BaseType> {
        if self.class(v).is_none_or(|c| self.held.contains(&c)) {
            return None;
        }
        self.result.types.get(v.0 as usize).cloned().flatten().filter(|t| matches!(t, BaseType::F32 | BaseType::F64))
    }
}

/// Classes whose widths stay with the legacy deciders for now (see module docs).
fn held_classes(fc: &FunctionConstraints, result: &SolverResult) -> BTreeSet<u32> {
    let view = View { result, held: BTreeSet::new() };
    let mut held = BTreeSet::new();
    // An index use in a class of another width is held, unless it is converted where it is used (an argument at a
    // `usize` formal: NUM-USIZE-TO-INT at the argument).
    let converted: BTreeSet<u32> = fc.index_use_keys.iter().map(|(_, v)| v.0).collect();
    for &v in &fc.index_uses {
        if view.int_type(v) != Some(BaseType::Usize) && !converted.contains(&v.0) {
            held.extend(view.class(v));
        }
    }
    // A name the walk met without a binding (a global, a closure parameter) is not named by the facts;
    // with lexical scoping every `let`/`for` binding has its own variable. A module constant with a
    // declared type is modelled (NUM-DECLARED).
    let mut bound: BTreeSet<&str> = fc.binding_vars.iter().map(|(n, _)| n.as_str()).collect();
    bound.extend(fc.unmodelled.declared_constants.iter().map(String::as_str));
    // Sorted: iteration order must not depend on the hash map (codegen determinism).
    let sorted_locals: std::collections::BTreeMap<String, ConstraintVar> = fc.var_map.locals.clone().into_iter().collect();
    for (name, v) in &sorted_locals {
        if !bound.contains(name.as_str()) {
            held.extend(view.class(*v));
        }
    }
    // The solver could not decide a class (two exact integer widths met): legacy.
    held.extend(result.width_conflicts.iter().filter_map(|v| view.class(*v)));
    // A negated literal in an unsigned class, or a loop binding of unknown element width: legacy.
    for &v in &fc.unmodelled.negated_literals {
        if matches!(view.int_type(v), Some(BaseType::U8 | BaseType::U16 | BaseType::U32 | BaseType::U64 | BaseType::U128 | BaseType::Usize)) {
            held.extend(view.class(v));
        }
    }
    let u = &fc.unmodelled;
    for &v in u.loop_bindings.iter().chain(&u.unknown_lengths).chain(&u.unknown_fields).chain(&u.opaque) {
        held.extend(view.class(v));
    }
    // Bindings of one name with different widths are not named by one NUM-LOCAL-WIDTH fact (see `publish`); their
    // classes are still published, and each read carries its own binding's width (NUM-USE-WIDTH).
    // The parts of a held container are held too (an opaque `Vec` holds its elements).
    loop {
        let more: Vec<u32> = result
            .parts
            .iter()
            .filter(|(c, _, _)| held.contains(c))
            .filter_map(|(_, _, v)| view.class(*v))
            .filter(|c| !held.contains(c))
            .collect();
        if more.is_empty() {
            break;
        }
        held.extend(more);
    }
    held
}

/// Each binding's solved width (or `None` for a non-integer binding), grouped by name.
fn binding_widths(fc: &FunctionConstraints, view: &View<'_>) -> std::collections::BTreeMap<String, Vec<(ConstraintVar, Option<BaseType>)>> {
    let mut out: std::collections::BTreeMap<String, Vec<(ConstraintVar, Option<BaseType>)>> = Default::default();
    for (name, v) in &fc.binding_vars {
        let w = view.int_type(*v);
        out.entry(name.clone()).or_default().push((*v, w));
    }
    out
}

pub fn publish(fc: &FunctionConstraints, result: &SolverResult, ir_fn: &mut IrFunction) {
    let held = held_classes(fc, result);
    let view = View { result, held };
    // A literal receiver of a numeric method (`2.pow(10)`) needs a width even when its class is defaulted.
    let receiver_classes: BTreeSet<u32> =
        fc.unmodelled.literal_receivers.iter().filter_map(|v| view.class(*v)).filter(|c| !view.held.contains(c)).collect();
    for (key, var) in &fc.literal_vars {
        let at_default = || {
            view.class(*var)
                .filter(|c| receiver_classes.contains(c))
                .and_then(|_| result.types.get(var.0 as usize).cloned().flatten())
                .filter(|t| matches!(t, BaseType::I64 | BaseType::F64))
        };
        if let Some(base) = view.published(*var).or_else(|| view.published_float(*var)).or_else(at_default) {
            ir_fn.literal_widths.insert(*key, base);
        }
    }
    for name in &fc.single_binding_locals {
        let Some(&var) = fc.var_map.locals.get(name) else { continue };
        let defaulted = result.defaulted.get(var.0 as usize).copied().unwrap_or(true);
        if !defaulted && matches!(result.types.get(var.0 as usize), Some(Some(BaseType::I64))) {
            ir_fn.solver_i64_locals.insert(name.clone());
        }
    }
    // NUM-LOCAL-WIDTH: a name all of whose bindings the solver fixed to one width.
    for (name, widths) in binding_widths(fc, &view) {
        if fc.unmodelled.borrowed_bindings.contains(&name) {
            continue;
        }
        let ws: Vec<Option<BaseType>> = widths.iter().map(|(v, _)| view.published(*v)).collect();
        if let Some(Some(w)) = ws.first() {
            if ws.iter().all(|x| x.as_ref() == Some(w)) {
                ir_fn.solver_local_widths.insert(name, w.clone());
            }
        }
    }
    // NUM-USE-WIDTH: each read of a local has its binding's width, also when the name is rebound with another width.
    // A binding that holds a borrow keeps its reference type (no width for its reads), as for NUM-LOCAL-WIDTH.
    // Only reads of `let`/pattern bindings: parameters keep their declared spelling paths.
    let borrowed: BTreeSet<u32> =
        fc.binding_vars.iter().filter(|(n, _)| fc.unmodelled.borrowed_bindings.contains(n)).map(|(_, v)| v.0).collect();
    let bindings: BTreeSet<u32> = fc.binding_vars.iter().map(|(_, v)| v.0).collect();
    for (key, var) in &fc.ident_uses {
        if borrowed.contains(&var.0) || !bindings.contains(&var.0) {
            continue;
        }
        if let Some(w) = view.published(*var) {
            ir_fn.use_widths.insert(*key, w);
        }
    }
    // NUM-USIZE-TO-INT: an index source in a class of another width is converted where it is produced.
    for (key, var) in &fc.index_sources {
        if let Some(w) = view.published(*var) {
            ir_fn.index_source_widths.insert(*key, w);
        }
    }
    // ... and a `usize` value passed to a std method's generic slot of another integer width (`out.push(i + 1)` on a
    // `Vec<int>`) is converted at the argument.
    // ... and an argument of another integer width at a `usize` formal is converted where it is passed.
    for (key, arg) in &fc.index_use_keys {
        if view.published(*arg).is_some_and(|w| is_integer_type(&w) && w != BaseType::Usize) {
            ir_fn.usize_conversions.insert(*key, BaseType::Usize);
        }
    }
    for (key, arg, slot) in &fc.element_stores {
        let Some(elem) = view.int_type(*slot) else { continue };
        if view.published(*arg) == Some(BaseType::Usize) && is_integer_type(&elem) && elem != BaseType::Usize {
            ir_fn.usize_conversions.insert(*key, elem);
        }
    }
}

#[cfg(test)]
mod tests {
    fn facts(src: &str, f: &str) -> crate::ir::node::IrFunction {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(program).expect("analyze");
        let module = crate::ir::IrPipeline::new().lower_to_ir(&analyzed, &registry, None);
        module.functions.into_iter().find(|x| x.name == f).expect("function")
    }

    use crate::ir::safety_type::BaseType;

    /// ADR-0028: a counter bounded by `len()` is published `usize`, with its literals and the `len()`.
    #[test]
    fn len_bounded_counter_publishes_usize_widths() {
        let f = facts("pub fn f(v: Vec<string>) -> bool {\n    let mut i = 0\n    while i < v.len() {\n        i = i + 1\n    }\n    true\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
        assert!(f.literal_widths.values().all(|w| *w == BaseType::Usize) && f.literal_widths.len() == 2, "{:?}", f.literal_widths);
        assert_eq!(f.index_source_widths.len(), 1);
    }

    /// NUM-USIZE-TO-INT: a `len()` that meets a declared `int` is converted at the call; its class is int.
    #[test]
    fn len_meeting_an_int_is_converted_at_the_call() {
        let f = facts("pub fn f(v: Vec<string>, n: int) -> int {\n    let mut t = v.len() + n\n    t = t + 1\n    t\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("t"), Some(&BaseType::I64));
        assert_eq!(f.index_source_widths.values().collect::<Vec<_>>(), vec![&BaseType::I64]);
    }

    /// The bounds of a text slice (`s[i..i + 1]` is `s.slice(i, i + 1)`, declared `usize` slots) keep the class of
    /// `i: int` and are converted at the argument: the literal is published `i64`, not left to the `i32` default.
    #[test]
    fn text_slice_bounds_keep_the_int_class() {
        let f = facts("pub fn f(s: string, i: int) -> string {\n    let c = s[i..i + 1]\n    c.to_string()\n}\n", "f");
        assert_eq!(f.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::I64], "{:?}", f.literal_widths);
        assert_eq!(f.usize_conversions.len(), 2, "{:?}", f.usize_conversions);
    }

    /// NUM-USIZE-TO-INT at the index: an index in a class of another width keeps that width and is converted where it
    /// indexes a sequence (`v[k]` prints `v[k as usize]`).
    #[test]
    fn index_use_of_an_int_class_is_converted_at_the_index() {
        let f = facts("pub fn f(v: Vec<string>, n: int) -> string {\n    let k = n + 1\n    v[k]\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("k"), Some(&BaseType::I64), "{:?}", f.solver_local_widths);
        assert_eq!(f.usize_conversions.values().collect::<Vec<_>>(), vec![&BaseType::Usize], "{:?}", f.usize_conversions);
    }

    /// A `usize` counter pushed into a returned `Vec<int>` is converted at the argument (it flows into the slot).
    #[test]
    fn usize_counter_pushed_into_int_vec_is_converted() {
        let f = facts("pub fn f(lines: Vec<string>) -> Vec<int> {\n    let mut out = Vec::new()\n    let mut i = 0\n    while i < lines.len() {\n        out.push(i + 1)\n        i = i + 1\n    }\n    out\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
        assert_eq!(f.usize_conversions.values().collect::<Vec<_>>(), vec![&BaseType::I64], "{:?}", f.usize_conversions);
    }

    /// Rebinding a name in a later scope (`for j in ..` then `let mut j = ..`) is a separate class;
    /// both bindings fix one width, so the name is published.
    #[test]
    fn rebound_name_with_one_width_is_published() {
        let f = facts("pub fn f(v: Vec<int>) -> int {\n    let mut s = 0\n    for j in 0..v.len() {\n        s = s + v[j]\n    }\n    let mut i = 0\n    while i < v.len() {\n        let mut j = i + 1\n        while j < v.len() {\n            j = j + 1\n        }\n        i = i + 1\n    }\n    s\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("j"), Some(&BaseType::Usize), "{:?}", f.solver_local_widths);
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
    }

    /// Bindings of one name with different widths cannot be named by one fact: held.
    #[test]
    fn rebound_name_with_two_widths_is_held() {
        let f = facts("pub fn f(n: i32, v: Vec<int>) -> i32 {\n    let x = n + 1\n    let mut k = 0\n    while k < v.len() {\n        k = k + 1\n    }\n    let x = k\n    n\n}\n", "f");
        assert!(f.solver_local_widths.get("x").is_none(), "{:?}", f.solver_local_widths);
    }

    /// NUM-USE-WIDTH: a counter bound twice, once against a `len()` (usize) and once against a `u32` it is passed as,
    /// has no single name width, but each read carries its own binding's width and the literals are published.
    #[test]
    fn rebound_counter_reads_carry_their_bindings_width() {
        let src = "fn deg(v: int) -> u32 {\n    3\n}\nfn at(v: int, i: u32) -> int {\n    v\n}\npub fn f(ids: Vec<int>, v: int) -> bool {\n    if ids.len() > 0 {\n        let mut i = 0\n        while i < ids.len() {\n            i = i + 1\n        }\n        return false\n    }\n    let d = deg(v)\n    let mut i = 0\n    while i < d {\n        let _ = at(v, i)\n        i = i + 1\n    }\n    true\n}\n";
        let f = facts(src, "f");
        assert!(f.solver_local_widths.get("i").is_none(), "{:?}", f.solver_local_widths);
        let first: Vec<_> = f.use_widths.iter().filter(|((l, _), _)| *l == 10 || *l == 11).map(|(_, w)| w.clone()).collect();
        let second: Vec<_> = f.use_widths.iter().filter(|((l, _), _)| (17..=19).contains(l)).map(|(_, w)| w.clone()).collect();
        assert!(!first.is_empty() && first.iter().all(|w| *w == BaseType::Usize), "{:?}", f.use_widths);
        assert!(second.iter().filter(|w| **w == BaseType::U32).count() >= 3, "{:?}", f.use_widths);
        assert!(f.literal_widths.values().any(|w| *w == BaseType::U32), "{:?}", f.literal_widths);
    }

    /// A local bound to a borrow (`&i`, `opt.unwrap_or(&0)`) holds a reference, not an integer: the
    /// solver publishes no width for it, so the backend keeps its reference type (and the deref at its use).
    #[test]
    fn borrowed_binding_has_no_local_width() {
        let f = facts("pub fn f(items: Vec<i32>) -> i32 {\n    let i = 1\n    let ri = &i\n    let n = items[i]\n    n\n}\n", "f");
        assert!(f.solver_local_widths.get("ri").is_none(), "{:?}", f.solver_local_widths);
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
        let g = facts("pub fn g(o: Option<u32>) -> u32 {\n    let c = o.unwrap_or(&0)\n    c\n}\n", "g");
        assert!(g.solver_local_widths.get("c").is_none(), "{:?}", g.solver_local_widths);
    }

    /// The same push when the Vec is returned inside `Ok(..)`: the declared payload types the Vec.
    #[test]
    fn usize_counter_pushed_into_int_vec_returned_in_ok_is_converted() {
        let f = facts("pub fn f(lines: Vec<string>) -> Result<Vec<int>, string> {\n    let mut out = Vec::new()\n    let mut i = 0\n    while i < lines.len() {\n        out.push(i + 1)\n        i = i + 1\n    }\n    Ok(out)\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
        assert_eq!(f.usize_conversions.values().collect::<Vec<_>>(), vec![&BaseType::I64], "{:?}", f.usize_conversions);
    }

    /// Option C C1: `parse::<int>()` then `unwrap_or(0)` is an `int` (the typed core surface), so a counter
    /// assigned from it is published `i64` with its literal.
    #[test]
    fn counter_assigned_from_parse_int_is_i64() {
        let f = facts("fn main() {\n    let line = \"5\"\n    let mut k = 0\n    k = line.trim().parse::<int>().unwrap_or(0)\n    println(\"${k}\")\n}\n", "main");
        assert_eq!(f.solver_local_widths.get("k"), Some(&BaseType::I64), "{:?}", f.solver_local_widths);
    }

    /// Option C C1: `map.get(k).unwrap_or(0)` on a `HashMap<string, int>` is the value type.
    #[test]
    fn map_get_unwrap_or_is_the_value_width() {
        let f = facts("use std::collections::HashMap\nfn main() {\n    let mut counts: HashMap<string, int> = HashMap::new()\n    let key = \"a\"\n    let n = counts.get(key).unwrap_or(0)\n    counts.insert(key, n + 1)\n}\n", "main");
        assert_eq!(f.solver_local_widths.get("n"), Some(&BaseType::I64), "{:?}", f.solver_local_widths);
    }

    /// Option C C1: a `u8` vector's `push` argument is a `u8` (element type from the surface).
    #[test]
    fn push_argument_takes_the_element_width() {
        let f = facts("pub fn f(v: Vec<u8>) -> Vec<u8> {\n    let mut out = v\n    let b = 7\n    out.push(b)\n    out\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("b"), Some(&BaseType::U8), "{:?}", f.solver_local_widths);
    }

    /// A module constant has its declared type: `for d in -1..1 { put(base + d) }` with `base = SIZE`
    /// (`const SIZE: i32`) is one `i32` class, published (was held as an unknown name, so the legacy
    /// path printed `-1_i64`).
    #[test]
    fn module_constant_types_its_class() {
        let src = "const SIZE: i32 = 64\nfn put(x: i32) {\n}\npub fn build() {\n    let base = SIZE\n    for d in -1..1 {\n        put(base + d)\n    }\n}\n";
        let f = facts(src, "build");
        assert_eq!(f.solver_local_widths.get("d"), Some(&BaseType::I32), "{:?}", f.solver_local_widths);
        assert!(f.literal_widths.values().all(|w| *w == BaseType::I32) && !f.literal_widths.is_empty(), "{:?}", f.literal_widths);
    }

    /// The float class: an unsuffixed float literal takes the width of its uses (an `f32` parameter, a
    /// returned accumulator, an enum payload); with no such use it is defaulted and not published.
    #[test]
    fn float_literals_take_the_width_of_their_uses() {
        let f = facts("pub fn f(x: f32) -> f32 {\n    x * 0.5\n}\n", "f");
        assert_eq!(f.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::F32], "{:?}", f.literal_widths);
        let acc = facts("pub fn g(a: bool) -> f32 {\n    let mut r = 0.0\n    if a {\n        r = r + 0.2\n    }\n    r\n}\n", "g");
        assert!(acc.literal_widths.len() == 2 && acc.literal_widths.values().all(|w| *w == BaseType::F32), "{:?}", acc.literal_widths);
        let open = facts("pub fn h() {\n    let z = 2.5\n    println(\"${z}\")\n}\n", "h");
        assert_eq!(open.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::F64], "{:?}", open.literal_widths);
    }

    /// Stage C2: a class no use fixes is the language default (`int` = i64), published like any other width.
    #[test]
    fn unconstrained_literals_publish_the_int_default() {
        let f = facts("pub fn f() -> bool {\n    let v = [3, 1, 2]\n    let k = 10\n    v.len() > 1 && k > 2\n}\n", "f");
        let widths: Vec<_> = f.literal_widths.values().filter(|w| **w != BaseType::Usize).collect();
        assert!(widths.len() == 5 && widths.iter().all(|w| **w == BaseType::I64), "{:?}", f.literal_widths);
    }

    /// Stage C2: a closure parameter is an element of the receiver; a literal beside it takes the element's width.
    #[test]
    fn closure_parameter_takes_the_receivers_element_width() {
        let f = facts("pub fn f(v: Vec<i32>) -> Vec<i32> {\n    v.iter().map(|x| x * 2).collect()\n}\n", "f");
        assert_eq!(f.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::I32], "{:?}", f.literal_widths);
        let fold = facts("pub fn g(v: Vec<u8>) -> u8 {\n    v.iter().fold(0, |a, x| a + x)\n}\n", "g");
        assert_eq!(fold.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::U8], "{:?}", fold.literal_widths);
    }

    /// Stage C2: tuple fields, `Some` payloads and pushed elements are parts of their containers.
    #[test]
    fn parts_of_declared_containers_type_their_literals() {
        let t = facts("fn take(p: (i32, u8)) -> u8 {\n    p.1\n}\npub fn f() -> u8 {\n    take((1, 2))\n}\n", "f");
        let mut ws: Vec<_> = t.literal_widths.values().cloned().collect();
        ws.sort_by_key(|w| format!("{w:?}"));
        assert_eq!(ws, vec![BaseType::I32, BaseType::U8], "{:?}", t.literal_widths);
        let o = facts("pub fn f(n: i16) -> Option<i16> {\n    if n > 0 { Some(n - 1) } else { None }\n}\n", "f");
        assert!(o.literal_widths.len() == 2 && o.literal_widths.values().all(|w| *w == BaseType::I16), "{:?}", o.literal_widths);
        let p = facts("pub fn f() -> Vec<u32> {\n    let mut r: Vec<u32> = Vec::new()\n    r.push(1)\n    r\n}\n", "f");
        assert_eq!(p.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::U32], "{:?}", p.literal_widths);
    }

    /// Stage C2: a payload inside a payload takes the formal's nested width (`Result::Ok(Some(42))` at a
    /// `Result<Option<i32>, string>` formal is an `i32`).
    #[test]
    fn nested_payload_takes_the_formals_width() {
        let f = facts("fn t(r: Result<Option<i32>, string>) -> bool {\n    true\n}\npub fn f() -> bool {\n    t(Result::Ok(Some(42)))\n}\n", "f");
        assert_eq!(f.literal_widths.values().collect::<Vec<_>>(), vec![&BaseType::I32], "{:?}", f.literal_widths);
        let m = facts("pub fn f() -> bool {\n    let mut m: HashMap<i32, i32> = HashMap::new()\n    m.insert(1, 2)\n    true\n}\n", "f");
        assert!(m.literal_widths.len() == 2 && m.literal_widths.values().all(|w| *w == BaseType::I32), "{:?}", m.literal_widths);
    }

    /// Stage C2: a closure parameter typed by the surface has that static type in the body: `s.len()` is a std
    /// length, converted where it meets the `int` element of the collected `Vec<int>`.
    #[test]
    fn closure_parameter_static_type_types_methods_in_the_body() {
        let f = facts("pub fn f() -> Vec<int> {\n    \"ab cd\".split(\" \").map(|s| s.len()).collect::<Vec<int>>()\n}\n", "f");
        assert_eq!(f.index_source_widths.values().collect::<Vec<_>>(), vec![&BaseType::I64], "{:?} {:?}", f.index_source_widths, f.literal_widths);
        let a = facts("type Pair<T> = (T, T)\nfn swap(p: Pair<int>) -> Pair<int> {\n    (p.1, p.0)\n}\npub fn f() -> Pair<int> {\n    swap((1, 2))\n}\n", "f");
        assert!(a.literal_widths.len() == 2 && a.literal_widths.values().all(|w| *w == BaseType::I64), "{:?}", a.literal_widths);
    }

    /// A built-in function displays its arguments: a value printed with `println` keeps its class published, so a
    /// literal beside an `int` call result is an `int` (`1 + pick(seed, 6)`).
    #[test]
    fn printed_values_stay_published() {
        let f = facts("fn pick(seed: int, n: int) -> int {\n    seed % n\n}\npub fn f() {\n    let seed = 12345\n    let n = 1 + pick(seed, 6)\n    println(n)\n}\n", "f");
        assert!(f.literal_widths.len() == 3 && f.literal_widths.values().all(|w| *w == BaseType::I64), "{:?}", f.literal_widths);
    }

    /// A value passed to a call the walk cannot type is opaque: its class (and the class of its elements) stays
    /// with the legacy deciders instead of taking the default.
    #[test]
    fn values_passed_to_unknown_calls_are_not_published() {
        let f = facts("pub fn f() {\n    let v = [1, 2]\n    mystery(v, 3)\n}\n", "f");
        assert!(f.literal_widths.is_empty(), "{:?}", f.literal_widths);
    }
}
