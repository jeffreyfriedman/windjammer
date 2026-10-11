//! `self::f()` (module-relative path call) and `self.f()` (method call on the value) are distinct
//! AST nodes and therefore resolve differently. Before the parser change both parsed to
//! `MethodCall { object: self, method: f }`, so the resolver offered method candidates for a path
//! (see docs/resolve-shadow-diffs.md history).
#[path = "resolve_support.rs"]
mod support;
use windjammer::resolve::{CallKind, Res};

const SRC: &str = r#"
fn f() -> int { 7 }

struct S {}

impl S {
    fn f(self) -> int { 1 }
    fn via_method(self) -> int { self.f() }
    fn via_path(self) -> int { self::f() }
}

mod m {
    pub fn f() -> int { 3 }
    pub fn g() -> int { self::f() }
}
"#;

#[test]
fn self_path_call_resolves_to_the_module_function() {
    let rp = support::resolve_one(SRC);
    let c = support::call(&rp, "S::via_path", "self::f");
    assert_eq!(c.kind, CallKind::Path, "{c:?}");
    assert_eq!(c.res, Res::Def(support::def(&rp, "f")), "{c:?}");
}

#[test]
fn self_method_call_resolves_to_method_candidates() {
    let rp = support::resolve_one(SRC);
    let c = support::call(&rp, "S::via_method", "f");
    assert_eq!(c.kind, CallKind::Method, "{c:?}");
    assert!(matches!(&c.res, Res::MethodCandidates(v) if v.contains(&support::def(&rp, "S::f"))), "{c:?}");
}

#[test]
fn self_path_inside_a_module_is_that_module() {
    let rp = support::resolve_one(SRC);
    let c = support::call(&rp, "m::g", "self::f");
    assert_eq!(c.res, Res::Def(support::def(&rp, "m::f")), "{c:?}");
}
