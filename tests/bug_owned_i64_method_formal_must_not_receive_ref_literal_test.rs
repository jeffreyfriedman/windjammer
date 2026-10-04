#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! P3.651: owned `i64` method formal must not receive `&99` at the call site.
//!
//! Product `wj-notes-api` `store.get(99)` tip emits `store.get(&99_i64)` → E0308
//! while formal is `id: i64`. Distinct from HashMap key borrow / P3.649 slice
//! `&usize`. Do not reshape notes tests.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn owned_i64_method_formal_must_not_receive_ref_literal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod store
pub mod check
"#,
    );
    test.add_file(
        "store.wj",
        r#"
pub struct Store {
    pub n: int,
}

impl Store {
    pub fn new() -> Store {
        Store { n: 1 }
    }

    pub fn get(self, id: int) -> bool {
        id == self.n
    }
}
"#,
    );
    test.add_file(
        "check.wj",
        r#"
use crate::store::Store

pub fn missing() -> bool {
    let store = Store::new()
    store.get(99)
}
"#,
    );

    let map = test
        .compile()
        .expect("P3.651 MultiFile compile should succeed");
    let check_rs = map.get("check.rs").cloned().unwrap_or_default();
    let store_rs = map.get("store.rs").cloned().unwrap_or_default();
    eprintln!("P3.651 store get sig:\n{}", store_rs.lines().find(|l| l.contains("fn get")).unwrap_or(""));
    eprintln!("P3.651 check.rs:\n{check_rs}");

    let bad = check_rs.contains("get(&99") || check_rs.contains(".get(&99");
    assert!(
        !bad,
        "P3.651 RED: owned i64 formal must receive bare 99, not &99:\n{check_rs}\n{store_rs}"
    );

    test.cargo_check()
        .expect("P3.651 RED: store.get(99) must cargo-check");
}
