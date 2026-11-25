//! f32 bit-exactness: every program of `tests/f32_parity` prints f32 results as `to_bits()` hex and f64 widenings
//! as `{:016x}`; the tree walker, the resolved engine, the resolved engine with every loop eager in machine code
//! (`WJ_JIT_LOOP_HOT=0`) and the COMPILED Rust program (`wj build --no-cargo` + rustc) must print the same bytes
//! (ADR-0030: an agent comparing `wj run` and `wj run --compiled` sees the same bits). Programs use only
//! operations whose f32 result Rust defines exactly (no libm function: those are ledger rows L1/L2).
//! Soundness: `docs/design/interp-f32-units.md`.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn programs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/f32_parity");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "wj")).collect();
    v.sort();
    v
}

fn interpreted(p: &Path, engine: &str, loop_hot: Option<&str>) -> String {
    let mut c = Command::new(WJ);
    c.env_remove("WJ_JIT_LOOP_HOT");
    if let Some(h) = loop_hot {
        c.env("WJ_JIT_LOOP_HOT", h);
    }
    let o = c.args(["run", "--interpret", "--no-lint", "--engine", engine]).arg(p).stdin(std::process::Stdio::null()).output().unwrap();
    assert!(o.status.success(), "{} ({engine}): {}{}", p.display(), String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into()
}

fn compiled(p: &Path) -> String {
    let scratch = Scratch::new("f32bits");
    let out = scratch.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(p).output().unwrap();
    assert!(b.status.success(), "wj build {}: {}", p.display(), String::from_utf8_lossy(&b.stderr));
    let rs = std::fs::read_dir(&out).unwrap().flatten().map(|e| e.path()).find(|f| f.extension().is_some_and(|e| e == "rs")).expect("no .rs emitted");
    let bin = scratch.join("prog");
    let r = Command::new("rustc").args(["--edition", "2021", "-A", "warnings", "-o"]).arg(&bin).arg(&rs).output().unwrap();
    assert!(r.status.success(), "rustc rejected {}: {}", p.display(), String::from_utf8_lossy(&r.stderr));
    let run = Command::new(&bin).output().unwrap();
    String::from_utf8_lossy(&run.stdout).into()
}

#[test]
fn f32_programs_print_the_same_bits_in_every_engine_and_compiled_rust() {
    let progs = programs();
    assert!(!progs.is_empty());
    for p in progs {
        let walker = interpreted(&p, "walker", None);
        let resolved = interpreted(&p, "resolved", None);
        let eager = interpreted(&p, "resolved", Some("0"));
        let rust = compiled(&p);
        assert!(!walker.is_empty(), "{} printed nothing", p.display());
        assert_eq!(walker, rust, "{}: walker differs from compiled Rust", p.display());
        assert_eq!(resolved, rust, "{}: resolved engine differs from compiled Rust", p.display());
        assert_eq!(eager, rust, "{}: eager machine-code loops differ from compiled Rust", p.display());
    }
}
