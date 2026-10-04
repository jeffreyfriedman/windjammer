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

//! P3.590c: product `BuildFingerprint::validate(source_dir: String)` keeps owned
//! (forwards to `generate(source_dir: String)`) but `check_build_freshness` emits
//! `validate(&source_dir)` → E0308.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
    eprintln!("P3.590c MultiFile emit:\n{rs}");
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
        "P3.590c RED: owned String param borrowed into owned validate formal:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn tip_out_build_fingerprint_must_not_borrow_source_dir() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/rendering/build_fingerprint.rs");
    let rs = std::fs::read_to_string(&game).unwrap_or_default();
    assert!(!rs.is_empty(), "missing {}", game.display());
    assert!(
        rs.contains("source_dir: String") && rs.contains("fn validate("),
        "expected owned validate formal"
    );
    let bad: Vec<_> = rs
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(".validate(") && l.contains("&source_dir"))
        .map(|(i, l)| format!("{}:{}", i + 1, l.trim()))
        .collect();
    assert!(
        bad.is_empty(),
        "P3.590c RED tip-out:\n  {}",
        bad.join("\n  ")
    );
}
