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

//! P3.590: owned `String` path formal must not receive `&path` / `&_temp` at call
//! sites. Product `gen/assets/loader.rs`: `load(..., path: String, ...)` but
//! `self.load(name.clone(), &path, size)` / `loader.load(_temp0, &_temp1, …)`.

use std::path::PathBuf;

#[test]
fn tip_out_loader_owned_string_path_must_not_borrow() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/assets/loader.rs");
    let rs = std::fs::read_to_string(&game).unwrap_or_default();
    assert!(
        !rs.is_empty(),
        "P3.590: missing gen/assets/loader.rs (run engine tip transpile)"
    );
    let owned_formal = rs.contains("path: String") && rs.contains("pub fn load(");
    assert!(owned_formal, "expected owned path: String formal:\n{}", &rs[..rs.len().min(500)]);
    let bad = rs.contains("&path") || rs.contains("&_temp1") || rs.contains("&_temp");
    assert!(
        !bad,
        "P3.590 RED: owned String path formal borrowed at call site in {}",
        game.display()
    );
}
