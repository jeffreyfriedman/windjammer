//! The generated manifest of a `--web`/wasm build must not pull the runtime's default `full` group, and when the
//! `crypto` group is pulled in (bcrypt -> getrandom 0.3) it must select getrandom's `wasm_js` backend by itself.

use std::path::{Path, PathBuf};
use windjammer::cargo_toml::generate_wasm_cargo_toml_with_config;

fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime")
}

/// Writes `main.rs` into a scratch crate dir and returns `(Cargo.toml, .cargo/config.toml if any)`.
fn wasm_manifest_for(lib_rs: &str) -> (String, Option<String>, tempfile::TempDir) {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("lib.rs"), lib_rs).unwrap();
    let cfg = windjammer::compiler_config::CompilerConfig {
        runtime_path: Some(runtime_dir()),
        ..Default::default()
    };
    generate_wasm_cargo_toml_with_config(t.path(), t.path(), &cfg).unwrap();
    let m = std::fs::read_to_string(t.path().join("Cargo.toml")).unwrap();
    let c = std::fs::read_to_string(t.path().join(".cargo/config.toml")).ok();
    (m, c, t)
}

#[test]
fn web_build_does_not_pull_default_runtime_features() {
    let (m, c, _t) = wasm_manifest_for("pub fn f() { windjammer_runtime::io::println(\"hi\"); }\n");
    assert!(m.contains("default-features = false"), "{m}");
    assert!(m.contains("features = [\"wasm\"]"), "{m}");
    assert!(!m.lines().any(|l| l.starts_with("getrandom")), "no crypto, no getrandom 0.3 pin: {m}");
    assert!(c.is_none(), "no config.toml when nothing needs it");
}

#[test]
fn web_build_with_crypto_selects_getrandom_wasm_js() {
    let (m, c, _t) = wasm_manifest_for("use windjammer_runtime::*;\npub fn f() {}\n");
    assert!(m.contains("[target.'cfg(target_arch = \"wasm32\")'.dependencies]"), "{m}");
    assert!(m.contains("getrandom = { version = \"0.3\", features = [\"wasm_js\"] }"), "{m}");
    let c = c.expect("config.toml written");
    assert!(c.contains("[target.wasm32-unknown-unknown]"), "{c}");
    assert!(c.contains("getrandom_backend=\\\"wasm_js\\\""), "{c}");
    toml::from_str::<toml::Value>(&m).expect("manifest parses");
    toml::from_str::<toml::Value>(&c).expect("config parses");
}

/// Real build for wasm32-unknown-unknown of the generated crate; skipped when the target is not installed.
#[test]
fn web_build_with_crypto_compiles_for_wasm32() {
    let installed = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-unknown-unknown"))
        .unwrap_or(false);
    let Some(target_root) = std::env::var_os("CARGO_TARGET_DIR") else { return };
    if !installed {
        eprintln!("skip: wasm32-unknown-unknown not installed");
        return;
    }
    let (_m, _c, t) = wasm_manifest_for("use windjammer_runtime::crypto;\npub fn h() { let _ = crypto::sha256(\"x\"); }\n#[no_mangle]\npub extern \"C\" fn boot() {}\n");
    let status = std::process::Command::new("cargo")
        .current_dir(t.path())
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .args(["build", "--target", "wasm32-unknown-unknown", "--release", "--target-dir"])
        .arg(PathBuf::from(target_root).join("wasm-getrandom-test"))
        .status()
        .unwrap();
    assert!(status.success(), "generated web crate must build for wasm32-unknown-unknown");
}
