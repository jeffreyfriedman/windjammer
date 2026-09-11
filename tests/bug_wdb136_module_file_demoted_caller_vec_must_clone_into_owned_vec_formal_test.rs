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

//! WDB-136: demoted caller `&Vec<T>` into owned `Vec<T>` formal (over-borrow) → E0308
//! (`expected Vec<DremelEncodedField>, found &Vec<DremelEncodedField>`).
//!
//! WindjammerDB CQ-C5 product shape (`document_dremel_*`):
//!   - `document_dremel_query_fields_by_ordinal(fields: Vec<…>, …) -> Vec<…>` stays owned
//!   - `document_dremel_index_build(fields: &Vec<…>, …)` demotes + passes bare `fields`
//!
//! Tip must either unify demotion OR emit `fields.clone()` at the owned call site.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod query
pub mod index
"#;

const QUERY: &str = r#"
pub struct EncodedField {
    pub ordinal: u32,
}

/// Consuming filter — product keeps owned Vec formal under multipass.
pub fn query_fields_by_ordinal(fields: Vec<EncodedField>, ordinal: u32) -> Vec<EncodedField> {
    let mut out: Vec<EncodedField> = Vec::new()
    for f in fields {
        if f.ordinal == ordinal {
            out.push(f)
        }
    }
    out
}
"#;

const INDEX: &str = r#"
use crate::query::EncodedField
use crate::query::query_fields_by_ordinal

pub struct OrdinalIndex {
    pub ordinal: u32,
    /// Vec<i64> (not usize) so this gate stays isolated from WDB-119 i64→usize.
    pub field_indices: Vec<i64>,
}

/// Cap leaf borrows `fields` into build → tip may demote formal to `&Vec`.
pub fn index_build(fields: Vec<EncodedField>, ordinal: u32) -> OrdinalIndex {
    let matched = query_fields_by_ordinal(fields, ordinal)
    let mut indices: Vec<i64> = Vec::new()
    let mut i: i64 = 0
    while i < (matched.len() as i64) {
        indices.push(i)
        i = i + 1
    }
    OrdinalIndex {
        ordinal: ordinal,
        field_indices: indices,
    }
}

pub fn index_cap() -> OrdinalIndex {
    let fields = vec![
        EncodedField { ordinal: 1 },
        EncodedField { ordinal: 1 },
        EncodedField { ordinal: 2 },
    ]
    index_build(fields, 1)
}
"#;

fn wdb136_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("query.wj", QUERY);
    test.add_file("index.wj", INDEX);
    test
}

#[test]
fn wdb136_module_file_demoted_caller_vec_must_clone_into_owned_vec_formal() {
    let test = wdb136_fixture();
    let map = test
        .compile()
        .expect("WDB-136 multipass compile should succeed (codegen may still be wrong)");
    let query_rs = map.get("query.rs").expect("query.rs");
    let index_rs = map.get("index.rs").expect("index.rs");

    let caller_demoted = index_rs.contains("fields: &Vec<EncodedField>")
        || index_rs.contains("fields: &Vec <EncodedField>");
    let callee_owned = {
        let i = query_rs.find("fn query_fields_by_ordinal").unwrap_or(0);
        let sl = &query_rs[i..query_rs.len().min(i + 140)];
        sl.contains("fields: Vec<EncodedField>") && !sl.contains("fields: &Vec")
    };
    let clones = index_rs.contains("query_fields_by_ordinal(fields.clone()");
    let bare = index_rs.contains("query_fields_by_ordinal(fields,") && !clones;
    let bad = caller_demoted && callee_owned && bare;

    eprintln!("WDB-136 query.rs:\n{query_rs}\nindex.rs:\n{index_rs}");
    eprintln!(
        "caller_demoted={caller_demoted} callee_owned={callee_owned} clones={clones} bare={bare} bad={bad}"
    );

    if bad {
        panic!(
            "WDB-136 RED: demoted caller &Vec into owned Vec formal must .clone(). \
             Product: document_dremel_query_fields_by_ordinal / document_dremel_index_build."
        );
    }

    test.cargo_check().expect(
        "WDB-136: demoted &Vec into owned Vec formal must compile (clone or unify demotion).",
    );
}
