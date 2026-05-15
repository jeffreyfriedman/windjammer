#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: E0382 / Copy registry alignment — empty structs and declaration order.
//!
//! Root causes fixed:
//! 1. `main.rs` PASS 0 skipped empty structs; they are Copy in Rust and in codegen.
//! 2. Analyzer Phase 0 used a single forward pass, so a struct referencing a later
//!    Copy struct was never marked Copy → wrong `self` inference → E0382.

#[path = "../common/test_utils.rs"]
mod test_utils;

/// E0382: empty struct is Copy; getter returning Copy type should use `&self` so the
/// receiver can be used again.
#[test]
fn test_empty_struct_copy_self_borrow_for_getter() {
    let source = r#"
pub struct Marker {}

impl Marker {
    pub fn tag(self) -> Marker {
        Marker {}
    }
}

fn main() {
    let m = Marker {}
    let _ = m.tag()
    let _ = m.tag()
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Must compile without E0382. Rust:\n{}", rs);
    assert!(
        rs.contains("fn tag(&self)") || rs.contains("fn tag(self)"),
        "Empty Marker is Copy; tag should take &self or self. Got:\n{}",
        rs
    );
}

/// E0382: `Wrapper` appears before `Inner` in source; fixed-point marks both Copy so
/// `inner_copy` can use `&self` and `w` is usable twice.
#[test]
fn test_copy_struct_forward_reference_declaration_order() {
    let source = r#"
pub struct Wrapper {
    inner: Inner
}

pub struct Inner {
    v: u32
}

impl Wrapper {
    pub fn inner_copy(self) -> Inner {
        self.inner
    }
}

fn main() {
    let w = Wrapper { inner: Inner { v: 1 } }
    let _ = w.inner_copy()
    let _ = w.inner_copy()
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(
        rs.contains("fn inner_copy(&self)"),
        "Wrapper should be Copy after Inner; method should be &self. Got:\n{}",
        rs
    );
    assert!(compiles, "Must compile without E0382. Rust:\n{}", rs);
}

/// E0507-style guard: registry must treat empty structs as Copy (matches codegen derive).
#[test]
fn test_empty_struct_derives_copy_in_output() {
    let source = r#"
pub struct UnitLike {}

pub fn main() {
    let _ = UnitLike {}
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(
        rs.contains("derive(") && rs.contains("Copy"),
        "Empty struct should auto-derive Copy like codegen. Got:\n{}",
        rs
    );
    assert!(compiles, "Must compile. Rust:\n{}", rs);
}
