//! A release binary panics on integer overflow exactly like a debug build and the interpreter (SPEC-NUM-002).
//! `[build] overflow_checks = false` in wj.toml and `wj build --unchecked-arithmetic` opt a project out.
//! Independent oracle: the specified outcome (exit status 101, a message containing `overflow`) and the Cargo
//! manifest text cargo itself reads.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const PROGRAM: &str = "fn main() {\n    let a = 9223372036854775807\n    println(\"before\")\n    let b = a + 1\n    println(\"${b}\")\n}\n";

fn project(wj_toml_extra: &str) -> Scratch {
    let d = Scratch::new("overflow-checks");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("wj.toml"), format!("[package]\nname = \"ovf\"\nversion = \"0.1.0\"\n{wj_toml_extra}")).unwrap();
    std::fs::write(d.join("src/main.wj"), PROGRAM).unwrap();
    d
}

fn manifest(wj_toml_extra: &str, flags: &[&str]) -> String {
    let d = project(wj_toml_extra);
    let o = Command::new(WJ).args(["build", "--no-cargo", "-o", "out"]).args(flags).arg(".").current_dir(d.path()).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    std::fs::read_to_string(d.join("out/Cargo.toml")).unwrap()
}

#[test]
fn release_profile_checks_overflow_by_default() {
    let m = manifest("", &[]);
    assert!(m.contains("[profile.release]") && m.contains("overflow-checks = true"), "{m}");
}

#[test]
fn project_and_flag_can_opt_out() {
    assert!(manifest("\n[build]\noverflow_checks = false\n", &[]).contains("overflow-checks = false"));
    assert!(manifest("", &["--unchecked-arithmetic"]).contains("overflow-checks = false"));
    assert!(manifest("\n[profile.release]\noverflow-checks = false\n", &[]).contains("overflow-checks = false"));
}

fn find_binary(out: &Path) -> Option<std::path::PathBuf> {
    let rel = out.join("target/release");
    std::fs::read_dir(rel).ok()?.flatten().map(|e| e.path()).find(|p| p.is_file() && p.extension().is_none() && p.file_name().is_some_and(|n| n == "ovf" || n == "main"))
}

/// Builds the program with `wj build --release` and runs the optimized binary.
fn run_release(wj_toml_extra: &str, flags: &[&str]) -> (String, String, Option<i32>) {
    let d = project(wj_toml_extra);
    let b = Command::new(WJ).args(["build", "--release", "-o", "out"]).args(flags).arg(".").current_dir(d.path()).output().unwrap();
    assert!(b.status.success(), "{}{}", String::from_utf8_lossy(&b.stdout), String::from_utf8_lossy(&b.stderr));
    let bin = find_binary(&d.join("out")).unwrap_or_else(|| panic!("no release binary under {:?}", d.join("out/target/release")));
    let o = Command::new(bin).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code())
}

#[test]
fn release_binary_panics_on_overflow() {
    let (out, err, code) = run_release("", &[]);
    assert_eq!(out, "before\n", "{err}");
    assert_eq!(code, Some(101), "{err}");
    assert!(err.contains("overflow"), "{err}");
}

#[test]
fn unchecked_release_binary_wraps() {
    let (out, _, code) = run_release("", &["--unchecked-arithmetic"]);
    assert_eq!(code, Some(0));
    assert_eq!(out, "before\n-9223372036854775808\n");
}
