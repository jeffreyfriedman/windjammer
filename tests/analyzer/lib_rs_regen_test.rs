use std::fs;
use std::process::Command;

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_lib_rs_regenerated_when_mod_wj_adds_module() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    // Initial mod.wj with one module
    fs::write(src.join("mod.wj"), "pub mod alpha\n").unwrap();
    fs::write(src.join("alpha.wj"), "pub fn hello() -> i32 {\n    42\n}\n").unwrap();

    // First build
    let status = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .status()
        .expect("wj build should succeed");
    assert!(status.success(), "first build failed");

    let lib_rs = out.join("lib.rs");
    assert!(lib_rs.exists(), "lib.rs should be created on first build");
    let content1 = fs::read_to_string(&lib_rs).unwrap();
    assert!(
        content1.contains("pub mod alpha"),
        "lib.rs should contain alpha module"
    );
    assert!(
        !content1.contains("pub mod beta"),
        "lib.rs should NOT contain beta module yet"
    );

    // Add a new module to mod.wj
    fs::write(src.join("mod.wj"), "pub mod alpha\npub mod beta\n").unwrap();
    fs::write(src.join("beta.wj"), "pub fn world() -> i32 {\n    99\n}\n").unwrap();

    // Rebuild (lib.rs already exists)
    let status = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .status()
        .expect("wj rebuild should succeed");
    assert!(status.success(), "rebuild failed");

    // lib.rs should now contain BOTH modules
    let content2 = fs::read_to_string(&lib_rs).unwrap();
    assert!(
        content2.contains("pub mod alpha"),
        "lib.rs should still contain alpha after rebuild"
    );
    assert!(
        content2.contains("pub mod beta"),
        "lib.rs should contain beta after rebuild (was: {})",
        content2
    );
}

#[test]
fn test_lib_rs_strips_use_super() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod alpha\n").unwrap();
    fs::write(src.join("alpha.wj"), "pub fn hello() -> i32 {\n    42\n}\n").unwrap();

    let status = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .status()
        .expect("wj build should succeed");
    assert!(status.success(), "build failed");

    let lib_rs = out.join("lib.rs");
    let content = fs::read_to_string(&lib_rs).unwrap();
    assert!(
        !content.contains("use super::*"),
        "lib.rs should NOT contain 'use super::*' (invalid at crate root)"
    );
}

#[test]
fn test_lib_rs_updated_when_module_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    // Initial build with two modules
    fs::write(src.join("mod.wj"), "pub mod alpha\npub mod beta\n").unwrap();
    fs::write(src.join("alpha.wj"), "pub fn a() -> i32 { 1 }\n").unwrap();
    fs::write(src.join("beta.wj"), "pub fn b() -> i32 { 2 }\n").unwrap();

    let status = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .status()
        .expect("wj build should succeed");
    assert!(status.success());

    let lib_rs = out.join("lib.rs");
    let c1 = fs::read_to_string(&lib_rs).unwrap();
    assert!(c1.contains("pub mod alpha"));
    assert!(c1.contains("pub mod beta"));

    // Remove beta from mod.wj
    fs::write(src.join("mod.wj"), "pub mod alpha\n").unwrap();

    let status = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .status()
        .expect("wj rebuild should succeed");
    assert!(status.success());

    let c2 = fs::read_to_string(&lib_rs).unwrap();
    assert!(
        c2.contains("pub mod alpha"),
        "alpha should still be in lib.rs"
    );
    assert!(
        !c2.contains("pub mod beta"),
        "beta should be REMOVED from lib.rs after mod.wj no longer declares it"
    );
}
