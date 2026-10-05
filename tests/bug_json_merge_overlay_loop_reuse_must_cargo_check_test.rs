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

//! P3.669: multipass owned `overlay: Value` reused in a while-loop into
//! `take_field(value: Value, …)` must cargo-check (borrow or clone-on-reuse).
//! Product `wj-json-util::merge_values` tip 21:42 emits `take_field(overlay, …)`
//! with owned formal → E0382 move on second iteration. Do not reshape the package.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

const CODEC: &str = include_str!("fixtures/library_multipass/codec_json_merge_overlay_loop.wj");

#[test]
fn json_merge_overlay_loop_reuse_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod codec
pub use codec::count_overlay_fields
"#,
    );
    project.add_file("codec.wj", CODEC);

    let map = project.compile().expect("merge overlay loop multipass compile");
    let codec_rs = map.get("codec.rs").expect("codec.rs");
    eprintln!("P3.669 codec.rs take_field / loop sites:\n{codec_rs}");

    // Accept either demoted `&Value` formal or clone-on-reuse at the call site.
    let take_sig_borrowed = codec_rs.contains("fn take_field(value: &Value")
        || codec_rs.contains("fn take_field(value: & Value");
    let call_clones_or_borrows = codec_rs.contains("take_field(overlay.clone()")
        || codec_rs.contains("take_field(&overlay")
        || codec_rs.contains("take_field(& overlay");
    assert!(
        take_sig_borrowed || call_clones_or_borrows,
        "P3.669 RED: owned overlay reused in loop into owned take_field must borrow \
         formal or clone/borrow at call site; emitted:\n{codec_rs}"
    );

    project
        .cargo_check()
        .expect("P3.669 multipass merge overlay loop reuse must cargo-check");
}

fn json_util_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-json-util");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-json-util");
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
fn json_util_product_merge_values_must_cargo_check() {
    let pkg = json_util_pkg().unwrap_or_else(|| {
        panic!(
            "wj-json-util src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
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
        .expect("wj build wj-json-util");
    assert!(
        build.status.success(),
        "P3.669 wj-json-util transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let take_owned = lib.contains("fn take_field(value: Value");
    let overlay_moved = lib
        .lines()
        .any(|l| l.contains("take_field(overlay,") && !l.contains("overlay.clone"));
    assert!(
        !(take_owned && overlay_moved),
        "P3.669 RED product: merge_values must not move owned overlay into owned \
         take_field across loop iterations:\n{lib}"
    );

    // Full cargo check of generated crate (path mirrors product build/).
    let status = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .status()
        .expect("cargo check");
    assert!(
        status.success(),
        "P3.669 RED: wj-json-util product merge_values must cargo-check"
    );
}
