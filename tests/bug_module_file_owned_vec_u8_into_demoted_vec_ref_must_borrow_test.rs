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

//! owned `Vec<u8>` local / helper return into demoted `&Vec<u8>` must auto-borrow.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod wire
pub mod exec
"#;

const WIRE: &str = r#"
pub fn finish_execute(response: Vec<u8>) -> int {
    response.len()
}

pub fn encode_rows() -> Vec<u8> {
    vec![1, 2, 3]
}
"#;

const EXEC: &str = r#"
use crate::wire::finish_execute
use crate::wire::encode_rows

pub fn run_local() -> int {
    let response = encode_rows()
    finish_execute(response)
}

pub fn run_temp() -> int {
    finish_execute(encode_rows())
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn module_file_owned_vec_u8_into_demoted_vec_ref_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("wire.rs:\n{wire_rs}\nexec.rs:\n{exec_rs}");

    let demoted = wire_rs.contains("response: &Vec<u8>")
        || wire_rs.contains("response: &Vec <u8>")
        || wire_rs.contains("response:&Vec<u8>");
    let bad_local = exec_rs.contains("finish_execute(response)")
        && !exec_rs.contains("finish_execute(&response)");
    let bad_temp = exec_rs.contains("finish_execute(encode_rows())")
        && !exec_rs.contains("finish_execute(&encode_rows()");
    let good = exec_rs.contains("finish_execute(&response)")
        || exec_rs.contains("finish_execute(&encode_rows()");

    if demoted && (bad_local || bad_temp) && !good {
        panic!(
            "RED demoted &Vec<u8> received owned local/temp. \
Got:\n{exec_rs}\n{wire_rs}"
        );
    }

    if demoted {
        assert!(
            !bad_local && (!exec_rs.contains("finish_execute(encode_rows())") || good),
            "demoted &Vec<u8> must borrow owned args. Got:\n{exec_rs}"
        );
    }
}
