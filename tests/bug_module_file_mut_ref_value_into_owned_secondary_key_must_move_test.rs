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

//! `&mut Value` into owned `Value` formal must clone/move.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod keying
pub mod index
"#;

const KEYING: &str = r#"
pub struct CellValue {
    pub n: int,
}

/// Owns indexed value.
pub fn secondary_key(table_id: int, index_id: int, indexed: CellValue, row_id: int) -> int {
    table_id + index_id + indexed.n + row_id
}
"#;

const INDEX: &str = r#"
use crate::keying::CellValue
use crate::keying::secondary_key

pub struct Cell {
    pub value: CellValue,
}

pub fn build_key(table_id: int, index_id: int, cell: Cell, row_id: int) -> int {
    secondary_key(table_id, index_id, cell.value, row_id)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("keying.wj", KEYING);
    test.add_file("index.wj", INDEX);
    test
}

#[test]
fn module_file_mut_ref_value_into_owned_must_not_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let keying_rs = map.get("keying.rs").expect("keying.rs");
    let index_rs = map.get("index.rs").expect("index.rs");

    eprintln!("keying.rs:\n{keying_rs}\nindex.rs:\n{index_rs}");

    let owned = {
        let i = keying_rs.find("fn secondary_key").unwrap_or(0);
        let sl = &keying_rs[i..keying_rs.len().min(i + 160)];
        (sl.contains("indexed: CellValue") || sl.contains("indexed:CellValue"))
            && !(sl.contains("indexed: &") || sl.contains("indexed:&"))
    };
    let bad = index_rs.contains("secondary_key(table_id, index_id, &mut cell.value")
        || index_rs.contains("secondary_key(table_id, index_id, &cell.value");
    let good = index_rs.contains("secondary_key(table_id, index_id, cell.value")
        || index_rs.contains("secondary_key(table_id, index_id, cell.value.clone()");

    if owned && bad {
        panic!(
            "RED owned Value formal received &mut/& field. \
Got:\n{index_rs}"
        );
    }
    if owned {
        assert!(
            good && !bad,
            "owned Value must get move/clone not &mut. Got:\n{index_rs}"
        );
    }
}
