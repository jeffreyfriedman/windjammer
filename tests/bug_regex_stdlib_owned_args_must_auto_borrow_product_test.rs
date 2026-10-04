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

//! P3.655: product `wj-regex` thin wrappers must auto-borrow into regex formals.
//!
//! ```ignore
//! pub fn is_match(pattern: string, text: string) -> … {
//!     regex.is_match(pattern, text)  // formals: &str, &str
//! }
//! ```
//! Tip keeps owned `String` formals and emits bare `regex::is_match(pattern, text)`
//! (also find/find_all/split) → E0308. `replace`/`escape` may already demote.
//! Distinct from P3.645/P3.654. Do not reshape with `.as_str()`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn regex_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-regex");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-regex");
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

fn thin_call_bare_owned(lib: &str, fn_name: &str, call: &str) -> bool {
    let marker = format!("pub fn {fn_name}(");
    let start = match lib.find(&marker) {
        Some(i) => i,
        None => return false,
    };
    let region = &lib[start..lib.len().min(start + 320)];
    let owned = region.contains("pattern: String") && region.contains("text: String");
    let borrowed = region.contains(&call.replace("(pattern, text)", "(&pattern, &text)"))
        || region.contains(&call.replace("(pattern, text)", "(&pattern, text)"))
        || region.contains(&call.replace("(pattern, text)", "(pattern, &text)"));
    owned && region.contains(call) && !borrowed
}

#[test]
fn regex_stdlib_owned_args_must_auto_borrow_product() {
    let pkg = regex_pkg().unwrap_or_else(|| {
        panic!(
            "wj-regex src not found by walking from {}",
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
        .expect("wj build wj-regex");
    assert!(
        build.status.success(),
        "P3.655 wj-regex transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.655 wj-regex lib.rs:\n{lib}");

    let bad_is_match = thin_call_bare_owned(&lib, "is_match", "regex::is_match(pattern, text)");
    let bad_find = thin_call_bare_owned(&lib, "find", "regex::find(pattern, text)");
    let bad_find_all = thin_call_bare_owned(&lib, "find_all", "regex::find_all(pattern, text)");
    let bad_split = thin_call_bare_owned(&lib, "split", "regex::split(pattern, text)");

    assert!(
        !(bad_is_match || bad_find || bad_find_all || bad_split),
        "P3.655 RED: regex::* must auto-borrow owned args (or demote formals):\n\
         is_match={bad_is_match} find={bad_find} find_all={bad_find_all} split={bad_split}\n{lib}"
    );
}
