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

//! FAILING REPRO — cross-crate owned handle reassignment in a loop emits `&mut`.
//!
//! Ecosystem `wj-pipeline` dogfood of `wj-sync`:
//! ```ignore
//! let mut tx = pair.0
//! while i < n {
//!     tx = send_int(tx, i)  // metadata: Owned IntSender → Owned IntSender
//! }
//! ```
//! Tip emits `send_int(&mut tx, i)` → E0308 expected `IntSender`, found `&mut IntSender`.
//! Same-crate reassignment of the identical pattern does **not** prefix `&mut`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_crate_owned_handle_loop_reassign_must_not_emit_mut_ref() {
    let tmp = TempDir::new().expect("tempdir");

    let pkg_src = tmp.path().join("sync_src");
    fs::create_dir_all(&pkg_src).expect("mkdir sync_src");
    fs::write(
        pkg_src.join("handle_pkg.wj"),
        r#"
pub struct Handle {
    pub n: int,
}

pub fn bump(h: Handle, delta: int) -> Handle {
    Handle { n: h.n + delta }
}
"#,
    )
    .unwrap();

    let pkg_gen = tmp.path().join("sync_gen");
    let pkg_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            pkg_src.to_str().unwrap(),
            "--output",
            pkg_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("handle_pkg build");
    assert!(
        pkg_build.status.success(),
        "handle_pkg library build failed:\n{}",
        String::from_utf8_lossy(&pkg_build.stderr)
    );

    let metadata_path = pkg_gen.join("metadata.json");
    assert!(metadata_path.exists(), "handle_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("domain")).expect("mkdir domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.handle_pkg]\npath = \"{}\"\npackage = \"sync_src\"\n",
            pkg_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("domain").join("run.wj"),
        r#"
use handle_pkg::Handle
use handle_pkg::bump

pub fn sum_bumps(n: int) -> int {
    let mut h = Handle { n: 0 }
    let mut i = 0
    while i < n {
        h = bump(h, 1)
        i = i + 1
    }
    h.n
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("domain").join("run.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("handle_pkg={}", metadata_path.display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("run.rs")).unwrap_or_else(|_| {
        fs::read_to_string(app_gen.join("domain").join("run.rs")).unwrap_or_default()
    });

    assert!(
        generated.contains("bump(h,")
            && !generated.contains("bump(&mut h")
            && !generated.contains("bump(&h"),
        "RED: cross-crate owned Handle loop reassign must pass by value.\ngenerated:\n{generated}"
    );
}
