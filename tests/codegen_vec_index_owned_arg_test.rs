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
))]

//! FAILING REPRO (LedgerKit row helpers — E0507 / E0308):
//!
//! `row_string(rows[0], …)` where the formal is owned `Row` must either clone
//! the indexed element (`rows[0].clone()`) or consistently take `&Row` — not
//! move out of the Vec index into an owned formal.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vec_index_into_owned_row_arg_must_clone_or_borrow_consistently() {
    let source = r#"
pub struct Row {
    pub label: string,
}

fn row_string(row: Row, suffix: string) -> string {
    row.label + ":" + suffix
}

pub fn first_labeled(rows: Vec<Row>, suffix: string) -> string {
    row_string(rows[0], suffix + "")
}

fn main() {
    let mut rows = Vec::new()
    rows.push(Row { label: "a" + "" })
    let _ = first_labeled(rows, "x" + "")
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);

    let owned_formal = generated.contains("fn row_string(row: Row")
        || generated.contains("fn row_string(mut row: Row");
    let borrowed_formal = generated.contains("fn row_string(row: &Row");

    if owned_formal {
        let call_clones_index = generated.contains("rows[0].clone()")
            || generated.contains("rows[0usize].clone()")
            || generated.contains("rows[(0) as usize].clone()")
            || generated.contains("rows[0i64 as usize].clone()");
        assert!(
            call_clones_index
                || generated.contains("row_string(rows[0].clone()")
                || (generated.contains("row_string(") && generated.contains(".clone()")),
            "owned Row formal requires clone of rows[0] (no E0507 move). Got:\n{generated}"
        );
        assert!(
            !generated.contains("row_string(rows[0],")
                || generated.contains("row_string(rows[0].clone()"),
            "must not move rows[0] into owned Row without .clone(). Got:\n{generated}"
        );
    } else if borrowed_formal {
        assert!(
            generated.contains("row_string(&rows[")
                || generated.contains("row_string(& rows["),
            "&Row formal must borrow the index consistently. Got:\n{generated}"
        );
    } else {
        panic!("row_string must take owned Row or &Row. Got:\n{generated}");
    }

    assert!(
        compiles,
        "generated Rust must compile (no E0507/E0308). Got:\n{generated}"
    );
}
