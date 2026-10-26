#![cfg(feature = "unstable_arenas")]
//! `std::arena` on the Rust backend: the shipped programs compile with `wj build`, link against
//! windjammer-runtime and print their independent `// expect:` lines; an escaping program is rejected
//! by `wj build` with WJ0160 before any Rust is emitted. Slow (builds the runtime once per target dir).

use std::path::{Path, PathBuf};
use std::process::Command;

fn expected(src: &str) -> String {
    src.lines()
        .filter_map(|l| l.strip_prefix("// expect: "))
        .map(|l| format!("{l}\n"))
        .collect()
}

fn work(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wj-arena-rust-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn shared_target() -> PathBuf {
    std::env::var_os("WJ_ARENA_TEST_TARGET")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("wj-arena-rust-target"))
}

fn build_and_run(src_path: &Path) -> String {
    let out = work(src_path.file_stem().unwrap().to_str().unwrap());
    let wj = env!("CARGO_BIN_EXE_wj");
    let st = Command::new(wj)
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(src_path)
        .output()
        .unwrap();
    assert!(
        st.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&st.stderr)
    );
    let run = Command::new("cargo")
        .args(["run", "-q"])
        .current_dir(&out)
        .env("CARGO_TARGET_DIR", shared_target())
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "generated program failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).to_string()
}

#[test]
fn arena_programs_run_on_the_rust_backend() {
    for name in ["particles", "scratch_labels"] {
        let p =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/arena_programs/{name}.wj"));
        let src = std::fs::read_to_string(&p).unwrap();
        assert_eq!(build_and_run(&p), expected(&src), "{name}");
    }
}

#[test]
fn wj_build_rejects_an_escaping_arena_value() {
    let dir = work("escape");
    let p = dir.join("escape.wj");
    std::fs::write(&p, "use std::arena::Scratch\nfn label() -> string {\n    let s = Scratch::new()\n    let t = s.str(\"x\")\n    return t\n}\nfn main() { println(label()) }\n").unwrap();
    let st = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(dir.join("out"))
        .arg(&p)
        .output()
        .unwrap();
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&st.stderr),
        String::from_utf8_lossy(&st.stdout)
    );
    assert!(!st.status.success(), "must not compile: {err}");
    assert!(err.contains("WJ0160") && err.contains("arena `s`"), "{err}");
}

#[test]
fn stale_handle_panics_in_a_debug_build_on_the_rust_backend() {
    let dir = work("stale");
    let p = dir.join("stale.wj");
    std::fs::write(&p, "use std::arena::Arena\nstruct P { x: int }\nfn main() {\n    let mut a = Arena::new()\n    let h = a.alloc(P { x: 1 })\n    let mut hs = Vec::new()\n    hs.push(h)\n    println(\"before\")\n    a.reset()\n    a.alloc(P { x: 2 })\n    println(\"${a.get(hs[0]).x}\")\n}\n").unwrap();
    let out = dir.join("out");
    let st = Command::new(env!("CARGO_BIN_EXE_wj")).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&p).output().unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
    let run = Command::new("cargo").args(["run", "-q"]).current_dir(&out).env("CARGO_TARGET_DIR", shared_target()).output().unwrap();
    assert!(!run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "before");
    assert!(String::from_utf8_lossy(&run.stderr).contains("stale arena handle"), "{}", String::from_utf8_lossy(&run.stderr));
}
