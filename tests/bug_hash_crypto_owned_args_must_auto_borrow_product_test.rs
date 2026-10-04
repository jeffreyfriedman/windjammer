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
    feature = "integration_tests",
))]

//! P3.654: product `wj-hash` thin wrappers must auto-borrow into crypto formals.
//!
//! ```ignore
//! pub fn verify_password(password: string, hash: string) -> … {
//!     crypto.verify_password(password, hash)  // formals: &str, &str
//! }
//! ```
//! Tip keeps owned `String` formals and emits bare
//! `crypto::verify_password(password, hash)` → E0308.
//! Distinct from P3.645 (csv stdlib). Do not reshape with `.as_str()`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn hash_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-hash");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-hash");
            if uncle.join("src").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn hash_crypto_owned_args_must_auto_borrow_product() {
    let pkg = hash_pkg().unwrap_or_else(|| {
        panic!(
            "wj-hash src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&pkg)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-hash");
    assert!(
        build.status.success(),
        "P3.654 wj-hash transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.654 wj-hash lib.rs:\n{lib}");

    let verify_region = {
        let start = lib.find("pub fn verify_password(").unwrap_or(0);
        &lib[start..lib.len().min(start + 280)]
    };
    let bare_owned = verify_region.contains("crypto::verify_password(password, hash)")
        && !verify_region.contains("crypto::verify_password(&password, &hash)")
        && !verify_region.contains("crypto::verify_password(&password, hash)")
        && !verify_region.contains("crypto::verify_password(password, &hash)");
    // Also RED if formals stay owned String while call is bare (same E0308).
    let owned_formals = verify_region.contains("password: String")
        && verify_region.contains("hash: String");

    assert!(
        !(bare_owned && owned_formals),
        "P3.654 RED: crypto::verify_password must auto-borrow owned args \
         (or demote formals to &str):\n{verify_region}\n{lib}"
    );
}
