//! Identity-based resolver: same-name collisions, scopes, locals, `Self`, enums, externs.
#[path = "resolve_support.rs"]
mod support;
use support::*;
use windjammer::resolve::{DefKind, Reason, Res};

#[test]
fn free_fn_does_not_compete_with_same_named_trait_method() {
    let rp = resolve_one(
        r#"
trait Shape { fn area(self) -> int }
struct Sq { s: int }
impl Shape for Sq { fn area(self) -> int { self.s } }
fn area(x: int) -> int { x }
fn main() { let a = area(3) }
"#,
    );
    assert_def(&rp, "main", "area", "area");
    assert!(matches!(def_kind(&rp, "area"), DefKind::Fn));
}

fn def_kind(rp: &windjammer::resolve::ResolvedProgram<'_>, path: &str) -> DefKind {
    rp.defs().get(def(rp, path)).kind
}

#[test]
fn free_fn_vs_inherent_method_and_assoc_path_in_same_module() {
    let rp = resolve_one(
        r#"
struct Counter { n: int }
impl Counter {
    fn new() -> Counter { Counter { n: 0 } }
    fn len(self) -> int { self.n }
}
fn len(x: int) -> int { x }
fn main() { let c = Counter::new(); let a = len(1); let b = c.len() }
"#,
    );
    assert_def(&rp, "main", "Counter::new", "Counter::new");
    // Free call `len(1)` resolves to the free fn; method call `c.len()` never competes with it.
    let free = rp
        .calls_in("main")
        .into_iter()
        .find(|c| c.callee == "len" && c.arg_count == 1)
        .unwrap();
    assert_eq!(free.res, Res::Def(def(&rp, "len")));
    let mc = rp
        .calls_in("main")
        .into_iter()
        .find(|c| c.callee == "len" && c.arg_count == 0)
        .unwrap();
    assert_eq!(
        mc.res,
        Res::MethodCandidates(vec![def(&rp, "Counter::len")])
    );
}

#[test]
fn two_modules_each_define_new_and_same_free_name() {
    let rp = resolve_files(&[
        ("", "mod a\nmod b\nfn main() { let x = a::make(); let y = b::make(); let p = a::P::new(); let q = b::P::new() }"),
        ("a", "pub struct P { x: int }\nimpl P { pub fn new() -> P { P { x: 1 } } }\npub fn make() -> int { 1 }"),
        ("b", "pub struct P { x: int }\nimpl P { pub fn new() -> P { P { x: 2 } } }\npub fn make() -> int { 2 }"),
    ]);
    assert_def(&rp, "main", "a::make", "a::make");
    assert_def(&rp, "main", "b::make", "b::make");
    assert_def(&rp, "main", "a::P::new", "a::P::new");
    assert_def(&rp, "main", "b::P::new", "b::P::new");
}

#[test]
fn trait_impls_for_multiple_types_resolve_by_type() {
    let rp = resolve_one(
        r#"
trait Make { fn make() -> int }
struct A {}
struct B {}
impl Make for A { fn make() -> int { 1 } }
impl Make for B { fn make() -> int { 2 } }
fn main() { let a = A::make(); let b = B::make(); let c = Make::make() }
"#,
    );
    assert_def(&rp, "main", "A::make", "<A as Make>::make");
    assert_def(&rp, "main", "B::make", "<B as Make>::make");
    assert_def(&rp, "main", "Make::make", "Make::make");
}

#[test]
fn inherent_wins_over_trait_impl_but_two_traits_are_ambiguous() {
    let rp = resolve_one(
        r#"
trait T1 { fn go() -> int }
trait T2 { fn go() -> int }
struct S {}
impl T1 for S { fn go() -> int { 1 } }
impl T2 for S { fn go() -> int { 2 } }
struct W {}
impl T1 for W { fn go() -> int { 1 } }
impl W { fn go() -> int { 3 } }
fn main() { let a = S::go(); let b = W::go() }
"#,
    );
    assert!(matches!(res(&rp, "main", "S::go"), Res::Ambiguous(v) if v.len() == 2));
    assert_def(&rp, "main", "W::go", "W::go");
    assert_eq!(rp.report.ambiguous, 1);
}

#[test]
fn locals_shadow_free_functions() {
    let rp = resolve_one(
        r#"
fn helper(x: int) -> int { x }
fn main() {
    let f = |x: int| x
    let helper = f
    let a = helper(1)
}
fn other(helper: int) { let b = helper(2) }
fn third() { let c = helper(3) }
"#,
    );
    assert_eq!(res(&rp, "main", "helper"), &Res::Local("helper".into()));
    assert_eq!(res(&rp, "other", "helper"), &Res::Local("helper".into()));
    assert_def(&rp, "third", "helper", "helper");
}

#[test]
fn self_paths_in_impl_and_trait() {
    let rp = resolve_one(
        r#"
trait Zero { fn zero() -> int
    fn twice() -> int { Self::zero() } }
struct P {}
impl P {
    fn base() -> int { 1 }
    fn go(self) -> int { Self::base() }
}
fn main() {}
"#,
    );
    assert_def(&rp, "P::go", "Self::base", "P::base");
    assert_def(&rp, "Zero::twice", "Self::zero", "Zero::zero");
}

#[test]
fn enum_variant_constructors_and_assoc_fns() {
    let rp = resolve_one(
        r#"
enum Shape { Circle(float), Unit }
impl Shape { fn unit() -> Shape { Shape::Unit } }
fn main() { let a = Shape::Circle(1.0); let b = Shape::unit(); let c = Some(1) }
"#,
    );
    assert_def(&rp, "main", "Shape::Circle", "Shape::Circle");
    assert_def(&rp, "main", "Shape::unit", "Shape::unit");
    assert_eq!(res(&rp, "main", "Some"), &Res::Prim("Some".into()));
    assert!(matches!(
        def_kind(&rp, "Shape::Circle"),
        DefKind::Variant { .. }
    ));
}

#[test]
fn extern_fns_resolve_as_extern_defs() {
    let rp = resolve_one("extern fn c_abs(x: int) -> int\nfn main() { let a = c_abs(1) }");
    assert_def(&rp, "main", "c_abs", "c_abs");
    assert!(matches!(def_kind(&rp, "c_abs"), DefKind::ExternFn));
}

#[test]
fn std_and_unknown_paths_are_classified() {
    let rp = resolve_one(
        "use std::fs\nfn main() { let a = fs::read(\"x\"); let v = Vec::new(); let z = nowhere(1) }",
    );
    assert!(is_external(res(&rp, "main", "fs::read")));
    assert_eq!(res(&rp, "main", "Vec::new"), &Res::Prim("Vec::new".into()));
    assert_eq!(
        res(&rp, "main", "nowhere"),
        &Res::Unresolved(Reason::NotFound("nowhere".into()))
    );
    assert_eq!(rp.report.unresolved, 2);
    assert_eq!(rp.report.diagnostics.len(), 1);
}

#[test]
fn method_candidates_filter_by_name_and_arity() {
    let rp = resolve_one(
        r#"
struct A {}
struct B {}
impl A { fn put(self, x: int) {} fn get(self) -> int { 1 } }
impl B { fn put(self, x: int, y: int) {} }
fn main(a: A) { a.put(1); a.get(); a.missing() }
"#,
    );
    let calls = rp.calls_in("main");
    let find = |n: &str| calls.iter().find(|c| c.callee == n).unwrap();
    assert_eq!(
        find("put").res,
        Res::MethodCandidates(vec![def(&rp, "A::put")])
    );
    assert_eq!(
        find("get").res,
        Res::MethodCandidates(vec![def(&rp, "A::get")])
    );
    assert!(matches!(
        find("missing").res,
        Res::Unresolved(Reason::NoMethodCandidate(_))
    ));
}

#[test]
fn generic_param_dispatch_is_not_a_not_found() {
    let rp = resolve_one("trait D { fn d() -> int }\nfn f<T: D>() -> int { T::d() }\nfn main() {}");
    assert!(matches!(
        res(&rp, "f", "T::d"),
        Res::Unresolved(Reason::GenericDispatch(_))
    ));
}

#[test]
fn def_paths_are_module_qualified() {
    let rp = resolve_files(&[
        ("", "mod net"),
        (
            "net",
            "pub fn send() {}\nstruct C {}\nimpl C { fn open() {} }",
        ),
    ]);
    assert!(rp.defs().find_one("net::send").is_some());
    assert!(rp.defs().find_one("net::C::open").is_some());
}
