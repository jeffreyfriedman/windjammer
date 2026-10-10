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

//! Regression: module file string ref into owned string must clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod parse
pub mod decode
"#;

const PARSE: &str = r#"
/// Owns the text.
pub fn parse_i64(text: string) -> Option<int> {
    if text.len() == 0 {
        return None
    }
    Some(text.len() as int)
}
"#;

const DECODE: &str = r#"
use crate::parse::parse_i64

pub fn pipe_field(line: string, index: int) -> string {
    let _ = index
    line
}

pub fn decode_text(wire: string) -> Option<int> {
    let status_text = pipe_field(wire, 0)
    // parse_i64(&status_text) while formal owned String — must clone/move.
    parse_i64(status_text)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("parse.wj", PARSE);
    test.add_file("decode.wj", DECODE);
    test
}

#[test]
fn module_file_string_ref_into_owned_string_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let parse_rs = map.get("parse.rs").expect("parse.rs");
    let decode_rs = map.get("decode.rs").expect("decode.rs");

    eprintln!("parse.rs:\n{parse_rs}\ndecode.rs:\n{decode_rs}");

    let callee_owned = {
        let i = parse_rs.find("fn parse_i64").unwrap_or(0);
        let sl = &parse_rs[i..parse_rs.len().min(i + 100)];
        (sl.contains("text: String") || sl.contains("text:String"))
            && !(sl.contains("text: &str") || sl.contains("text:&str") || sl.contains("text: &String"))
    };
    let bad = decode_rs.contains("parse_i64(&status_text)")
        || decode_rs.contains("parse_i64(&priority_text)");
    let good = decode_rs.contains("parse_i64(status_text)")
        || decode_rs.contains("parse_i64(status_text.clone()");

    if callee_owned && bad {
        panic!(
            "RED: owned String formal received &String. \
             Got:\n{decode_rs}\n{parse_rs}"
        );
    }

    if callee_owned {
        assert!(
            good && !bad,
            "owned String formal must not get &String. Got:\n{decode_rs}"
        );
    }
}
