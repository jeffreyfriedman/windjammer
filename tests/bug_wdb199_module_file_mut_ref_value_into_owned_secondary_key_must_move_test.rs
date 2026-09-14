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

//! WDB-199: `&mut Value` into owned `Value` formal must clone/move.
//!
//! Product residual (~2× Value←&mut Value), gen secondary_index:
//!   `relational_secondary_key(…, indexed: Value, …)`
//!   call `relational_secondary_key(…, &mut cell.value, …)` → E0308.
//! Signature-driven — owned Value formal must not receive `&mut`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod keying
pub mod index
"#;

const KEYING: &str = r#"
pub struct CellValue {
    pub n: int,
}

/// Owns indexed value (product relational_secondary_key).
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
    // Product: secondary_key(…, &mut cell.value, …) into owned — must move/clone
    secondary_key(table_id, index_id, cell.value, row_id)
}
"#;

fn wdb199_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("keying.wj", KEYING);
    test.add_file("index.wj", INDEX);
    test
}

#[test]
fn wdb199_module_file_mut_ref_value_into_owned_must_not_borrow() {
    let test = wdb199_fixture();
    let map = test
        .compile()
        .expect("WDB-199 multipass compile should succeed");
    let keying_rs = map.get("keying.rs").expect("keying.rs");
    let index_rs = map.get("index.rs").expect("index.rs");

    eprintln!("WDB-199 keying.rs:\n{keying_rs}\nindex.rs:\n{index_rs}");

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
            "WDB-199 RED: owned Value formal received &mut/& field. \
             Product: relational_secondary_key(…, &mut cell.value). Got:\n{index_rs}"
        );
    }
    if owned {
        assert!(
            good && !bad,
            "WDB-199: owned Value must get move/clone not &mut. Got:\n{index_rs}"
        );
    }
}

#[test]
fn wdb199_product_secondary_must_not_pass_mut_ref_into_owned_key() {
    // Product residual is in gitignored gen/ (tip-out may already be clean).
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen_index = gen.join("relational/relational_secondary_index_port.rs");
    let tip_index = tip.join("relational_secondary_index_port.rs");
    if !gen_index.exists() {
        eprintln!("WDB-199: skip — gen secondary_index missing");
        return;
    }
    let gen_text = std::fs::read_to_string(&gen_index).expect("gen index");
    let bad = gen_text.contains("relational_secondary_key(")
        && gen_text.contains("&mut cell.value");
    if tip_index.exists() {
        let tip_text = std::fs::read_to_string(&tip_index).unwrap_or_default();
        eprintln!(
            "WDB-199 tip-out clean={}",
            !(tip_text.contains("&mut cell.value"))
        );
    }
    eprintln!("WDB-199 product gen bad={} path={}", bad, gen_index.display());
    assert!(
        !bad,
        "WDB-199 RED: product gen still passes &mut Value into owned secondary_key. {}",
        gen_index.display()
    );
}
