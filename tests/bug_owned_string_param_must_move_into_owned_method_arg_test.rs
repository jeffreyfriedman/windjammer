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
    feature = "codegen_tests",
))]

//! (forwards to `generate(source_dir: String)`) but `check_build_freshness` emits
//! `validate(&source_dir)` → E0308.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Fingerprint {
    pub hash: string,
}

impl Fingerprint {
    pub fn generate(source_dir: string) -> Fingerprint {
        Fingerprint { hash: source_dir }
    }

    pub fn validate(self, source_dir: string) -> bool {
        let _current = Fingerprint::generate(source_dir)
        true
    }
}

pub fn check_freshness(source_dir: string) {
    let fp = Fingerprint { hash: "x" }
    let _ok = fp.validate(source_dir)
}
"#;

#[test]
fn owned_string_param_must_move_into_owned_method_arg() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("MultiFile emit:\n{rs}");
    let validate_sig = rs
        .lines()
        .find(|l| l.contains("fn validate(") && l.contains("source_dir"))
        .unwrap_or("")
        .to_string();
    assert!(
        validate_sig.contains("source_dir: String"),
        "validate formal must stay owned String (forwards to generate):\n{validate_sig}\n{rs}"
    );
    let bad: Vec<_> = rs
        .lines()
        .filter(|l| l.contains(".validate(") && l.contains("&source_dir"))
        .collect();
    assert!(
        bad.is_empty(),
        "RED: owned String param borrowed into owned validate formal:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
