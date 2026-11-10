//! An incremental library build must not inflate the signature registry: unchanged files contribute only
//! their own cached signatures, so the registry after touching one file equals the registry of a full build
//! (a flagship-size project went from about 6,000 to 985,000 signatures and 25-40 minute builds when every
//! unchanged file merged the whole cache again).

use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const MODULES: usize = 14;
const FUNCTIONS: usize = 6;

fn write_project(dir: &std::path::Path) {
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(dir.join("wj.toml"), "[package]\nname = \"regsize\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/mod.wj\"\n").unwrap();
    let mut root = String::new();
    for m in 0..MODULES {
        root.push_str(&format!("pub mod m{m}\n"));
        let mut text = String::new();
        for f in 0..FUNCTIONS {
            text.push_str(&format!("pub fn f{m}_{f}(text: string, n: int) -> int {{\n    n + text.len() as int\n}}\n\n"));
        }
        std::fs::write(src.join(format!("m{m}.wj")), text).unwrap();
    }
    std::fs::write(src.join("mod.wj"), root).unwrap();
}

/// Every registry size the build logs, one per generated file.
fn build(dir: &std::path::Path) -> Vec<usize> {
    let out = Command::new(WJ)
        .args(["build", "--no-cargo", "--no-lint", "--module-file", "-o"])
        .arg(dir.join("out"))
        .arg(dir.join("src/mod.wj"))
        .env("WJ_DEBUG_CODEGEN_FILE", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stderr)
        .split("registry≈")
        .skip(1)
        .filter_map(|s| s.split_whitespace().next()?.parse().ok())
        .collect()
}

#[test]
fn touching_one_file_does_not_multiply_the_registry() {
    let d = TempDir::new().unwrap();
    write_project(d.path());
    let full = *build(d.path()).iter().max().expect("a full build logs its registry size");
    let touched = d.path().join("src/m3.wj");
    let mut text = std::fs::read_to_string(&touched).unwrap();
    text.push_str("pub fn extra(n: int) -> int {\n    n\n}\n");
    std::fs::write(&touched, text).unwrap();
    let incremental = build(d.path());
    let inc = *incremental.iter().max().expect("an incremental build logs its registry size");
    assert!(inc <= full + 20, "full build registry {full}, incremental {inc}");
}
