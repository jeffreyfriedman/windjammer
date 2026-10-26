//! `rustc-error` oracle: the error code and message regex must persist. Uses a stand-in compiler
//! that emits a tiny crate (real `cargo check` judges it), so the test needs no runtime crate.

#[path = "common/reduce_fixtures.rs"]
mod fx;

use std::path::Path;

fn fake_compiler(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("fake-wj.sh");
    // E0308 when `* 7919` is present, E0425 when `* 104729` is present, else a clean crate.
    let script = r#"#!/bin/sh
out="$6"
mkdir -p "$out/src"
printf '[package]\nname = "fake"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n' > "$out/Cargo.toml"
body=''
if grep -rqE '\* *7919' "$2"; then body="$body let x: i32 = \"s\"; let _ = x;"; fi
if grep -rqE '\* *104729' "$2"; then body="$body let _ = missing_name;"; fi
body="fn main() { $body }"
printf '%s\n' "$body" > "$out/src/main.rs"
"#;
    std::fs::write(&p, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn program_with_both_constructs() -> Vec<(String, String)> {
    let mut files = fx::synthetic_program(8, Some(3));
    files.push((
        "extra.wj".into(),
        "pub fn other(x: int) -> int {\n    let y = x * 104729\n    y\n}\n".into(),
    ));
    files
}

#[test]
fn rustc_error_oracle_keeps_the_requested_code_and_drops_the_other_bug() {
    let d = tempfile::tempdir().unwrap();
    let (src, out) = (d.path().join("src"), d.path().join("out"));
    fx::write_program(&src, &program_with_both_constructs());
    let fake = fake_compiler(d.path());
    let r = fx::reduce(
        &src,
        &out,
        &[
            "--oracle",
            "rustc-error E0308:mismatched",
            "--wj-bin",
            fake.to_str().unwrap(),
            "--jobs",
            "2",
        ],
    );
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let all: String = fx::read_tree(&out)
        .into_iter()
        .filter(|(p, _)| p.ends_with(".wj"))
        .map(|t| t.1)
        .collect();
    assert!(all.contains("7919"), "E0308 trigger kept:\n{all}");
    assert!(
        !all.contains("104729"),
        "the unrelated E0425 trigger is dropped:\n{all}"
    );
}

#[test]
fn rustc_error_oracle_distinguishes_codes() {
    let d = tempfile::tempdir().unwrap();
    let (src, out) = (d.path().join("src"), d.path().join("out"));
    fx::write_program(&src, &program_with_both_constructs());
    let fake = fake_compiler(d.path());
    let r = fx::reduce(
        &src,
        &out,
        &[
            "--oracle",
            "rustc-error E0425",
            "--wj-bin",
            fake.to_str().unwrap(),
        ],
    );
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let all: String = fx::read_tree(&out)
        .into_iter()
        .filter(|(p, _)| p.ends_with(".wj"))
        .map(|t| t.1)
        .collect();
    assert!(all.contains("104729") && !all.contains("7919"), "{all}");
}

#[test]
fn rustc_error_message_regex_that_never_matches_is_rejected_on_the_input() {
    let d = tempfile::tempdir().unwrap();
    let (src, out) = (d.path().join("src"), d.path().join("out"));
    fx::write_program(&src, &program_with_both_constructs());
    let fake = fake_compiler(d.path());
    let r = fx::reduce(
        &src,
        &out,
        &[
            "--oracle",
            "rustc-error E0308:no such message",
            "--wj-bin",
            fake.to_str().unwrap(),
        ],
    );
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("does not hold"));
}
