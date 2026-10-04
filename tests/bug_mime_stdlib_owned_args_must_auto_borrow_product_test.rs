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

//! P3.657: product `wj-mime` thin wrappers must auto-borrow into mime formals.
//!
//! ```ignore
//! pub fn is_text(mime_type: string) -> bool { mime.is_text(mime_type) }
//! ```
//! Tip keeps owned `String` and emits bare `mime::is_text(mime_type)` → E0308
//! (`&str` expected). `from_extension`/`from_path` may already demote.
//! Distinct from P3.654/655 (crypto/regex). Do not reshape with `.as_str()`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn mime_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-mime");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-mime");
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

fn thin_predicate_bare_owned(lib: &str, fn_name: &str) -> bool {
    let marker = format!("pub fn {fn_name}(");
    let start = match lib.find(&marker) {
        Some(i) => i,
        None => return false,
    };
    let region = &lib[start..lib.len().min(start + 220)];
    let owned = region.contains("mime_type: String");
    let call = format!("mime::{fn_name}(mime_type)");
    let borrowed = region.contains(&format!("mime::{fn_name}(&mime_type)"));
    owned && region.contains(&call) && !borrowed
}

#[test]
fn mime_stdlib_owned_args_must_auto_borrow_product() {
    let pkg = mime_pkg().unwrap_or_else(|| {
        panic!(
            "wj-mime src not found by walking from {}",
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
        .expect("wj build wj-mime");
    assert!(
        build.status.success(),
        "P3.657 wj-mime transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.657 wj-mime lib.rs:\n{lib}");

    let bad = ["is_text", "is_image", "is_audio", "is_video"]
        .iter()
        .any(|name| thin_predicate_bare_owned(&lib, name));

    assert!(
        !bad,
        "P3.657 RED: mime::is_* must auto-borrow owned args (or demote formals):\n{lib}"
    );
}
