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

//! Impl methods with read-only `string` params must receive `&owned_string` at call sites,
//! not `.to_string()` (dogfooding: squad_tactics → Squad::new).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn impl_new_string_params_borrow_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "squad.wj",
        r#"
pub struct Squad {
    id: string,
}

impl Squad {
    pub fn new(id: string, leader_id: string) -> Squad {
        Squad { id: id }
    }
}
"#,
    );
    test.add_file(
        "caller.wj",
        r#"
use squad::Squad

pub fn make_squad(squad_id: string, leader_id: string) -> Squad {
    Squad::new(squad_id, leader_id)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("caller.rs").expect("caller.rs");
    assert!(
        rs.contains("Squad::new(&squad_id") || rs.contains("Squad::new( &squad_id"),
        "must borrow owned String args for &str impl params. Got:\n{rs}"
    );
    assert!(
        !rs.contains("Squad::new(squad_id.to_string()"),
        "must not spuriously to_string for &str callee. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
