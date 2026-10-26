#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! The interpreter feature registry (`src/interpreter/features.rs`) is executable: every entry's probe
//! program is run through the real interpreter and must behave as registered. `docs/INTERPRETER.md` is
//! generated from the same table (`wj-docs`), so the document cannot claim what the engine does not do.

use windjammer::interpreter::features::{Feature, Status, FEATURES};
use windjammer::interpreter::{ErrorKind, Interpreter};

#[derive(Debug, PartialEq)]
enum Outcome {
    Printed(String),
    Panicked(String),
    Unsupported(String),
    Other(String),
}

/// Run a probe the way `wj run` does: as a program file loaded through the shared module system and
/// resolver (`load_program_at`, which includes the preflight), then `main`.
fn run_probe(src: &str) -> Outcome {
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("main.wj");
            std::fs::write(&file, &src).unwrap();
            let mut it = Interpreter::new_capturing();
            let r = it.load_program_at(&file).and_then(|_| it.call_main());
            match r {
                Ok(_) => Outcome::Printed(it.get_output().trim_end_matches('\n').to_string()),
                Err(e) => match e.kind {
                    ErrorKind::Panic => Outcome::Panicked(e.message),
                    ErrorKind::Unsupported => Outcome::Unsupported(e.message),
                    _ => Outcome::Other(e.message),
                },
            }
        })
        .unwrap()
        .join()
        .unwrap_or_else(|_| Outcome::Other("interpreter panicked".into()))
}

/// The `wj run` fallback contract (ADR-0019): every `Unsupported` feature is detected at load time by the
/// preflight, before `main` runs, and no `Supported` feature is.
#[test]
fn unsupported_features_are_caught_before_main_and_supported_ones_are_not() {
    let mut bad = Vec::new();
    for f in FEATURES {
        let src = f.probe.to_string();
        let caught = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let dir = tempfile::tempdir().unwrap();
                let file = dir.path().join("main.wj");
                std::fs::write(&file, &src).unwrap();
                let mut it = Interpreter::new_capturing();
                match it.load_program_at(&file) {
                    Err(e) => windjammer::interpreter::preflight::is_preflight(&e),
                    Ok(()) => false,
                }
            })
            .unwrap()
            .join()
            .unwrap();
        if caught != (f.status == Status::Unsupported) {
            bad.push(format!("{} ({:?}): preflight caught = {}", f.id, f.status, caught));
        }
    }
    assert!(bad.is_empty(), "fallback trigger drifted from the registry:\n{}", bad.join("\n"));
}

fn conforms(f: &Feature, got: &Outcome) -> bool {
    match (f.status, got) {
        (Status::Supported, Outcome::Printed(out)) => {
            !f.expect.starts_with("panic: ") && out == f.expect
        }
        (Status::Supported, Outcome::Panicked(m)) => f
            .expect
            .strip_prefix("panic: ")
            .is_some_and(|w| m.contains(w)),
        (Status::Unsupported, Outcome::Unsupported(m)) => m.contains(f.expect),
        _ => false,
    }
}

#[test]
fn every_registered_feature_behaves_as_registered() {
    let bad: Vec<String> = FEATURES
        .iter()
        .filter_map(|f| {
            let got = run_probe(f.probe);
            (!conforms(f, &got)).then(|| {
                format!(
                    "{} ({:?}): expected {:?}, got {got:?}",
                    f.id, f.status, f.expect
                )
            })
        })
        .collect();
    assert!(
        bad.is_empty(),
        "registry drifted from the interpreter:\n{}",
        bad.join("\n")
    );
}

#[test]
fn registry_is_not_trivially_small_and_has_both_statuses() {
    assert!(
        FEATURES
            .iter()
            .filter(|f| f.status == Status::Supported)
            .count()
            >= 20
    );
    assert!(FEATURES.iter().any(|f| f.status == Status::Unsupported));
}

/// Mutation self-tests: the harness must reject wrong expectations and flipped statuses.
#[test]
fn harness_detects_drift() {
    let sup = FEATURES
        .iter()
        .find(|f| f.status == Status::Supported && !f.expect.starts_with("panic: "))
        .unwrap();
    let got = run_probe(sup.probe);
    assert!(conforms(sup, &got));
    let wrong = Feature {
        expect: "definitely not the output",
        ..*sup
    };
    assert!(
        !conforms(&wrong, &got),
        "a wrong expectation must be caught"
    );
    let flipped = Feature {
        status: Status::Unsupported,
        ..*sup
    };
    assert!(
        !conforms(&flipped, &got),
        "a feature that works but is registered Unsupported must be caught"
    );
    let uns = FEATURES
        .iter()
        .find(|f| f.status == Status::Unsupported)
        .unwrap();
    let got = run_probe(uns.probe);
    assert!(conforms(uns, &got));
    let flipped = Feature {
        status: Status::Supported,
        ..*uns
    };
    assert!(
        !conforms(&flipped, &got),
        "an unsupported feature registered as Supported must be caught"
    );
}
