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

//! P3.562: multi-file `wj build --module-file --output <temp>/<dir>` must not
//! copy sibling `*.rs` from the parent of `--output` into the package or
//! declare them as modules.
//!
//! Single-file `--library` isolates can false-GREEN. Product `wj-migrate`
//! (lib + db_apply + db_status) with leftover `/tmp/p3522-test-copy.rs` got
//! `pub mod p3522-test-copy` in `lib.rs`/`mod.rs` → rustc E0432. Tip multi-file
//! isolate copies `wj_p3562_stray_sentinel.rs` + other temp-root `*.rs`.

use std::fs;
use std::process::Command;

#[test]
fn module_file_output_must_not_import_tmp_sibling_rs() {
    let src = tempfile::TempDir::new().expect("src");
    fs::write(
        src.path().join("lib.wj"),
        "pub mod helper\npub fn answer() -> int { 42 }\n",
    )
    .unwrap();
    fs::write(src.path().join("helper.wj"), "pub fn n() -> int { 1 }\n").unwrap();

    let tmp = std::env::temp_dir();
    let sentinel_name = "wj_p3562_stray_sentinel.rs";
    let sentinel = tmp.join(sentinel_name);
    fs::write(&sentinel, "pub fn stray() {}\n").unwrap();

    let out = tmp.join(format!("wj_p3562_mf_out_{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.path().to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    let _ = fs::remove_file(&sentinel);
    assert!(
        build.status.success(),
        "P3.562 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let mod_rs = fs::read_to_string(out.join("mod.rs")).unwrap_or_default();
    let copied = out.join(sentinel_name).exists();
    eprintln!("P3.562 module-file emit:\nlib:\n{lib}\nmod:\n{mod_rs}\ncopied={copied}");

    let _ = fs::remove_dir_all(&out);

    assert!(
        !copied
            && !lib.contains("wj_p3562_stray_sentinel")
            && !mod_rs.contains("wj_p3562_stray_sentinel"),
        "P3.562 RED: module-file output must not pull sibling temp *.rs into the package:\ncopied={copied}\nlib:\n{lib}\nmod:\n{mod_rs}"
    );
}
