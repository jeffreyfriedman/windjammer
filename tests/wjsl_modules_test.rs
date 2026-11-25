//! `wj build --target wgsl` on `.wjsl` files: text includes (S0) and `use crate::m::item` modules (S1).
//! Design: docs/design/wjsl-modules.md.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Run `wj build FILE --target wgsl` and return (success, wgsl-or-stderr).
fn build_wjsl(entry: &Path, out_dir: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build"])
        .arg(entry)
        .args(["--target", "wgsl", "--no-cargo", "--output"])
        .arg(out_dir)
        .output()
        .expect("run wj");
    if !output.status.success() {
        return (false, String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stem = entry.file_stem().unwrap().to_string_lossy().to_string();
    (true, fs::read_to_string(out_dir.join(format!("{stem}.wgsl"))).expect("wgsl output"))
}

#[test]
fn s0_build_resolves_text_include() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("x.wjsl"), "fn helper(a: f32) -> f32 {\n    return a * 2.0;\n}\n").unwrap();
    fs::write(
        dir.path().join("main.wjsl"),
        "use \"x.wjsl\"\n@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n    return vec4<f32>(helper(1.0), 0.0, 0.0, 1.0);\n}\n",
    )
    .unwrap();
    let (ok, out) = build_wjsl(&dir.path().join("main.wjsl"), &dir.path().join("out"));
    assert!(ok, "build failed: {out}");
    assert!(out.contains("fn helper"), "{out}");
}
