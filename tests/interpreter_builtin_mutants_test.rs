//! Mutation checks for the interpreter's shared builtin classes (drift gate, step `builtin-mutants`).
//!
//! The walker and the resolved engine share the integer, float, string, format, sequence, map, option, iterator and
//! JSON builtins, so a break in one of them moves both engines together and the walker-vs-resolved comparison cannot
//! see it; only the compiled output can. Each program of `tests/drift_corpus/mutants/<class>.wj` therefore carries
//! the output the compiled program prints (`<class>.expected`, worked out from Rust's semantics). In debug builds
//! `WJ_JIT_MUTANT=<class>` deliberately breaks that class (`Interpreter::mutant`), and the check shows the program's
//! output then differs from the expected one under both engines, then that the restored builtin prints it again.
//! A class with a program but no mutant (or a mutant that changes nothing) fails here.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const CLASSES: &[&str] = &["int", "float", "format", "string", "seq", "map", "option", "iter", "json"];

fn run(class: &str, engine: &str, mutant: Option<&str>) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/drift_corpus/mutants/{class}.wj"));
    let mut c = Command::new(WJ);
    c.args(["run", "--interpret", "--no-lint", "--engine", engine]).arg(&p).stdin(std::process::Stdio::null());
    c.env_remove("WJ_JIT_MUTANT").env_remove("WJ_JIT");
    if let Some(m) = mutant {
        c.env("WJ_JIT_MUTANT", m);
    }
    let o = c.output().unwrap();
    format!("exit {:?}\n{}", o.status.code(), String::from_utf8_lossy(&o.stdout))
}

fn expected(class: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/drift_corpus/mutants/{class}.expected"));
    format!("exit Some(0)\n{}", std::fs::read_to_string(p).unwrap())
}

#[test]
fn every_builtin_class_program_has_a_check() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/drift_corpus/mutants");
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.unwrap().path().file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    found.sort();
    found.dedup();
    let mut want: Vec<String> = CLASSES.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(found, want, "tests/drift_corpus/mutants and CLASSES list different classes");
}

#[test]
fn restored_builtins_print_the_compiled_output() {
    for class in CLASSES {
        for engine in ["walker", "resolved"] {
            assert_eq!(run(class, engine, None), expected(class), "{class} under {engine} differs from the compiled output");
        }
    }
}

/// Debug builds only: release builds ignore `WJ_JIT_MUTANT`.
#[test]
#[cfg(debug_assertions)]
fn every_builtin_class_check_is_load_bearing() {
    for class in CLASSES {
        for engine in ["walker", "resolved"] {
            let mutated = run(class, engine, Some(class));
            assert_ne!(mutated, expected(class), "mutant `{class}` survived under {engine}");
            // the other classes' mutants leave this program alone
            let other = CLASSES.iter().find(|c| *c != class).unwrap();
            assert_eq!(run(class, engine, Some(other)), expected(class), "mutant `{other}` changed the `{class}` program");
        }
    }
}
