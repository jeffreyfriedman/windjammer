//! `std::time::Duration` / `Instant`: the compiled program (linked against windjammer-runtime) and the interpreter
//! print the same lines, and the generated manifest needs no calendar dependency (no `datetime` feature, no chrono).
//! Slow: builds the runtime once per target dir.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::{Path, PathBuf};
use std::process::Command;

fn expected(src: &str) -> String {
    src.lines().filter_map(|l| l.strip_prefix("// expect: ")).map(|l| format!("{l}\n")).collect()
}

fn shared_target() -> PathBuf {
    std::env::var_os("WJ_STD_TIME_TEST_TARGET").map(PathBuf::from).unwrap_or_else(|| scratch::shared_cache_dir("std-time-target"))
}

/// The literal's width comes from the declared formal (`from_millis(u64)` on the core surface), not from the std module's
/// placeholder `struct Duration` (which, registered as a program struct, made it `10_i32`). Fast: no cargo.
#[test]
fn a_duration_literal_takes_the_declared_u64_width() {
    let out = Scratch::new("std-time-width");
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/std_time_programs/duration_surface.wj");
    let st = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(&p)
        .env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1")
        .output()
        .unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
    let rs = std::fs::read_to_string(out.join("duration_surface.rs")).unwrap();
    assert!(rs.contains("Duration::from_millis(10_u64)"), "from_millis takes a u64:\n{rs}");
}

#[test]
fn duration_program_is_the_same_interpreted_and_compiled() {
    same_interpreted_and_compiled("duration_surface.wj", true);
}

/// `time.parse_duration_ms(text)` with an owned `string`: the runtime declares it in `time.rs` (ungated, no chrono; it was in `time/calendar.rs`, which `time.rs`
/// re-exported with `pub use calendar::*;`), so its `text: &str` formal is read under `time::parse_duration_ms` and the
/// argument is borrowed (rustc E0308 expected `&str`, found `String`).
#[test]
fn parse_duration_of_an_owned_string_is_the_same_interpreted_and_compiled() {
    same_interpreted_and_compiled("parse_duration_owned_text.wj", true);
}

fn same_interpreted_and_compiled(program: &str, no_calendar: bool) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/std_time_programs").join(program);
    let want = expected(&std::fs::read_to_string(&p).unwrap());
    let wj = env!("CARGO_BIN_EXE_wj");

    let interp = Command::new(wj).args(["run", "--interpret"]).arg(&p).env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1").output().unwrap();
    assert_eq!(String::from_utf8_lossy(&interp.stdout), want, "interpreter");

    let out = Scratch::new("std-time-duration");
    let st = Command::new(wj)
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(&p)
        .env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1")
        .output()
        .unwrap();
    assert!(st.status.success(), "wj build failed: {}", String::from_utf8_lossy(&st.stderr));
    let manifest = std::fs::read_to_string(out.join("Cargo.toml")).unwrap();
    assert!(!no_calendar || !manifest.contains("datetime") && !manifest.contains("chrono"), "no calendar dependency for durations:\n{manifest}");
    let run = Command::new("cargo").args(["run", "-q"]).current_dir(&out).env("CARGO_TARGET_DIR", shared_target()).output().unwrap();
    assert!(run.status.success(), "generated program failed: {}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout), want, "compiled");
}
