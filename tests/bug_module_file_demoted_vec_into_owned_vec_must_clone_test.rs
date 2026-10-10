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
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod wire
pub mod serve
"#;

const WIRE: &str = r#"
/// Owns the startup bytes.
pub fn decode_startup(buf: Vec<u8>) -> int {
    buf.len() as int
}
"#;

const SERVE: &str = r#"
use crate::wire::decode_startup

/// Multi-use read-only probe demotes formal toward `&Vec<u8>`.
pub fn buf_len(buf: Vec<u8>) -> int {
    buf.len() as int
}

pub fn on_startup(buf: Vec<u8>) -> int {
    let _n = buf_len(buf)
    decode_startup(buf)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn module_file_demoted_vec_into_owned_vec_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("wire.rs:\n{wire_rs}\nserve.rs:\n{serve_rs}");

    let callee_owned = {
        let i = wire_rs.find("fn decode_startup").unwrap_or(0);
        let sl = &wire_rs[i..wire_rs.len().min(i + 120)];
        (sl.contains("buf: Vec<u8>") || sl.contains("buf:Vec<u8>"))
            && !(sl.contains("buf: &Vec") || sl.contains("buf:&Vec"))
    };
    let caller_demoted = {
        let i = serve_rs.find("fn on_startup").unwrap_or(0);
        let sl = &serve_rs[i..serve_rs.len().min(i + 100)];
        sl.contains("buf: &Vec<u8>") || sl.contains("buf:&Vec<u8>")
    };
    let call_ok = serve_rs.contains("decode_startup(buf.clone()")
        || serve_rs.contains("decode_startup((*buf).clone()")
        || serve_rs.contains("decode_startup(buf.to_vec()")
        || serve_rs.contains("decode_startup((*buf).to_vec()");
    let call_bad = serve_rs.contains("decode_startup(buf)")
        && !serve_rs.contains("decode_startup(buf.clone()")
        && !serve_rs.contains("decode_startup(buf.to_vec()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "RED demoted &Vec<u8> passed into owned decode_startup without clone. \
Got:\n{serve_rs}\n{wire_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &Vec into owned Vec must clone. Got:\n{serve_rs}"
        );
    }
}
