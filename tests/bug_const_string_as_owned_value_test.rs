#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! A module `const NAME: string` lowers to `&'static str`. Wherever an owned `String` is
//! required (owned formal, `vec![..]` element, struct field, `let` with owned annotation),
//! the compiler must convert it; no `.clone()` of a `&str` and no user `+ ""` shim.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn check(src: &str) {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", src);
    test.cargo_check().unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn const_into_owned_formal_with_extra_args() {
    check(
        r#"
pub const KIND_A: string = "a"
pub const KIND_B: string = "b"

pub fn tag(kind: string, n: int) -> string { kind + "-" + "x" }
pub fn pass(kind: string) -> string { kind }

pub fn run() -> int {
    let s = tag(KIND_A, 1)
    let t = pass(KIND_B)
    let u = tag(KIND_A, 2)
    let w = pass(KIND_B)
    s.len() + t.len() + u.len() + w.len()
}
"#,
    );
}

#[test]
fn const_in_vec_literal_of_owned_strings() {
    check(
        r#"
pub const ONE: string = "1"
pub const TWO: string = "2"

pub fn all() -> Vec<string> {
    let v: Vec<string> = vec![ONE, TWO]
    v
}

pub fn direct() -> Vec<string> {
    vec![ONE, TWO]
}
"#,
    );
}

#[test]
fn const_into_struct_field_and_assignment() {
    check(
        r#"
pub const STATUS_OPEN: string = "open"
pub const STATUS_DONE: string = "done"

pub struct Item { pub status: string, pub label: string }

pub fn make() -> Item {
    Item { status: STATUS_OPEN, label: STATUS_DONE }
}

pub fn close(item: Item) -> Item {
    let mut it = item
    it.status = STATUS_DONE
    it
}
"#,
    );
}

#[test]
fn const_into_push_and_comparison() {
    check(
        r#"
pub const CLASS_READ: string = "read"

pub fn collect() -> Vec<string> {
    let mut out: Vec<string> = Vec::new()
    out.push(CLASS_READ)
    out
}

pub fn is_read(c: string) -> bool {
    c == CLASS_READ
}
"#,
    );
}

#[test]
fn const_from_another_module_into_owned_slots() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod names\npub mod user\n");
    test.add_file(
        "names.wj",
        r#"
pub const ROLE_ADMIN: string = "admin"
pub const ROLE_GUEST: string = "guest"
"#,
    );
    test.add_file(
        "user.wj",
        r#"
use crate::names::{ROLE_ADMIN, ROLE_GUEST}

pub struct User { pub role: string }

pub fn roles() -> Vec<string> {
    vec![ROLE_ADMIN, ROLE_GUEST]
}

pub fn label(role: string, n: int) -> string { role + "!" }

pub fn describe() -> string {
    let a = label(ROLE_ADMIN, 1)
    let b = label(ROLE_ADMIN, 2)
    a + b
}

pub fn guest() -> User {
    let mut u = User { role: ROLE_ADMIN }
    u.role = ROLE_GUEST
    u
}
"#,
    );
    test.cargo_check().unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn local_binding_reused_after_move_is_still_cloned_next_to_consts() {
    check(
        r#"
pub const PREFIX: string = "p"

pub fn take(s: string) -> int { s.len() }

pub fn reuse(owned: string) -> int {
    let a = take(owned)
    let b = take(owned)
    let c = take(PREFIX)
    let d = take(PREFIX)
    a + b + c + d
}
"#,
    );
}
