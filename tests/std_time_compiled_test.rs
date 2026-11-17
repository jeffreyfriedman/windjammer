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

#[test]
fn duration_program_is_the_same_interpreted_and_compiled() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/std_time_programs/duration_surface.wj");
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
    assert!(!manifest.contains("datetime") && !manifest.contains("chrono"), "no calendar dependency for durations:\n{manifest}");
    let run = Command::new("cargo").args(["run", "-q"]).current_dir(&out).env("CARGO_TARGET_DIR", shared_target()).output().unwrap();
    assert!(run.status.success(), "generated program failed: {}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout), want, "compiled");
}
