#![cfg(feature = "unstable_arenas")]
//! `std::arena` on every backend (ADR-0025): the same programs run on Rust, Go, JavaScript, TypeScript and the
//! interpreter and print the same independent `// expect:` lines; a stale handle panics on every backend.
//! Go and JS use the Windjammer-source prelude (`src/codegen/portable/prelude/arena.wj`).

#[path = "common/conformance/mod.rs"]
mod conformance;

use conformance::backends::{compile_and_run, Backend, Outcome};
use std::path::{Path, PathBuf};

fn wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn work(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wj-arena-be-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn expected(src: &str) -> Vec<String> {
    src.lines().filter_map(|l| l.strip_prefix("// expect: ")).map(str::to_string).collect()
}

fn stdout_of(o: &Outcome) -> Result<(Vec<String>, i32), String> {
    match o {
        Outcome::BuildFailed(e) => Err(e.clone()),
        Outcome::Ran(_, exec) => Ok((exec.stdout.lines().map(str::to_string).collect(), exec.code)),
    }
}

fn program(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/arena_programs").join(format!("{name}.wj"))).unwrap()
}

#[test]
fn go_and_js_ts_run_the_arena_programs() {
    for name in ["particles", "scratch_labels"] {
        let src = program(name);
        for backend in [Backend::Go, Backend::Js, Backend::Ts] {
            let got = stdout_of(&compile_and_run(&wj(), backend, &work(&format!("{name}-{backend:?}")), &src));
            assert_eq!(got.map(|(o, c)| (o, c)), Ok((expected(&src), 0)), "{name} on {backend:?}");
        }
    }
}

const STALE: &str = "use std::arena::Arena\nstruct P { x: int }\nfn main() {\n    let mut a = Arena::new()\n    let h = a.alloc(P { x: 1 })\n    let mut hs = Vec::new()\n    hs.push(h)\n    println(\"before\")\n    a.reset()\n    a.alloc(P { x: 2 })\n    println(\"${a.get(hs[0]).x}\")\n}\n";

#[test]
fn stale_handles_panic_on_go_and_js() {
    for backend in [Backend::Go, Backend::Js] {
        match compile_and_run(&wj(), backend, &work(&format!("stale-{backend:?}")), STALE) {
            Outcome::Ran(_, exec) => {
                assert_eq!(exec.stdout.trim(), "before", "{backend:?}");
                assert_ne!(exec.code, 0, "{backend:?} must fail");
                assert!(exec.stderr.contains("stale arena handle"), "{backend:?}: {}", exec.stderr);
            }
            Outcome::BuildFailed(e) => panic!("{backend:?}: {e}"),
        }
    }
}
