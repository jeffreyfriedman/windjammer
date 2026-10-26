//! `wj test --interpret` gives each run a private scratch directory (`env::temp_dir()`), granted for reading
//! and writing; nothing else outside the project is, and env/network stay opt-in.

use std::fs;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run_test(src: &str) -> (bool, String) {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("scratch_test.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["test", "--interpret"]).arg(&f).current_dir(d.path()).env_remove("WJ_CAPS").output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

#[test]
fn tests_write_and_read_inside_the_scratch_dir() {
    let (ok, out) = run_test(
        r#"use std::fs
use std::env
@test
fn scratch_round_trip() {
    let p = "${env::temp_dir()}/note.txt"
    fs::write(p, "hello").unwrap()
    assert_eq(fs::read_to_string(p).unwrap(), "hello")
}
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn the_scratch_dir_is_private_to_the_run_and_removed_afterwards() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("scratch_test.wj");
    let marker = d.path().join("where.txt");
    fs::write(
        &f,
        format!(
            "use std::fs\nuse std::env\n#[test]\nfn record() {{\n    fs::write(\"where.txt\", env::temp_dir()).unwrap()\n}}\n"
        )
        .replace("#[test]", "@test"),
    )
    .unwrap();
    let o = Command::new(WJ).args(["test", "--interpret"]).arg(&f).current_dir(d.path()).env_remove("WJ_CAPS").output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
    let dir = fs::read_to_string(&marker).unwrap();
    assert!(dir.contains("wj-scratch-"), "{dir}");
    assert!(!std::path::Path::new(&dir).exists(), "scratch dir should be removed after the run: {dir}");
}

#[test]
fn writing_elsewhere_in_system_temp_is_denied() {
    let other = std::env::temp_dir().join("wj-scratch-test-outside.txt");
    let (ok, out) = run_test(&format!(
        "use std::fs\n@test\nfn outside() {{\n    fs::write(\"{}\", \"x\").unwrap()\n}}\n",
        other.display()
    ));
    assert!(!ok, "{out}");
    assert!(out.contains("fs.write"), "{out}");
    assert!(!other.exists());
}

#[test]
fn env_stays_opt_in() {
    let (ok, out) = run_test("use std::env\n@test\nfn home() {\n    let _ = env::var(\"HOME\")\n}\n");
    assert!(!ok, "{out}");
    assert!(out.contains("[capabilities]"), "{out}");
}
