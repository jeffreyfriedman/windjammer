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

//! P3.572: `wj build path/to/mod.wj --module-file` must keep inline `pub struct` /
//! `impl` bodies from the source `mod.wj` in generated `mod.rs`.
//!
//! Product: ports `Host.tick_playable` vanished from `gen/ports/mod.rs` after a
//! ports-only tip transpile (double `--module-file` regen deleted `_mod_items`
//! then rewrote declarations-only).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn module_file_mod_wj_inline_struct_must_emit() {
    let root = TempDir::new().expect("tempdir");
    let ports = root.path().join("ports");
    fs::create_dir_all(&ports).unwrap();
    fs::write(
        ports.join("mod.wj"),
        r#"
pub mod host_helpers

pub struct Host {
    pub ticks: int,
}

impl Host {
    pub fn tick_playable(self) {
        self.ticks = self.ticks + 1
    }
}
"#,
    )
    .unwrap();
    fs::write(ports.join("host_helpers.wj"), "pub fn helper() -> int { 1 }\n").unwrap();

    let out = root.path().join("gen");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            ports.join("mod.wj").to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.572 transpile failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let mod_rs = fs::read_to_string(out.join("mod.rs")).unwrap_or_default();
    let lib_rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.572 mod.rs:\n{mod_rs}\nlib.rs:\n{lib_rs}");

    let has_host = mod_rs.contains("struct Host") || lib_rs.contains("struct Host");
    let has_tick = mod_rs.contains("tick_playable") || lib_rs.contains("tick_playable");
    assert!(
        has_host && has_tick,
        "P3.572 RED: --module-file must emit inline mod.wj Host + tick_playable:\nmod.rs:\n{mod_rs}\nlib.rs:\n{lib_rs}"
    );
}
