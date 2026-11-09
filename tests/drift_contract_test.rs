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

//! The interpreter's trust contract (docs/testing/INTERPRETER_CONTRACT.md): output equal to the compiled output
//! except for the ledgered divergences. This test keeps the ledger honest without running anything: it may not
//! grow past its ceiling, every line is well formed and carries a reason, and ids are unique. The drift gate
//! (scripts/wj/drift_gate.wj) is what compares behaviour; this is the cheap tripwire that fails in the default
//! test run when someone adds a divergence without raising the ceiling in the same diff.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ledger_lines(text: &str) -> Vec<&str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect()
}

fn check_ledger(text: &str, ceiling: usize) -> Result<(), String> {
    let lines = ledger_lines(text);
    if lines.len() > ceiling {
        return Err(format!(
            "the divergence ledger has {} lines, above its ceiling of {ceiling} (tests/drift_corpus/ledger_ceiling.txt): a new divergence needs a finding in docs/drift/findings.md and a deliberate ceiling change",
            lines.len()
        ));
    }
    let mut seen = HashSet::new();
    for l in &lines {
        let f: Vec<&str> = l.split(" | ").collect();
        if f.len() != 3 || f.iter().any(|x| x.trim().is_empty()) {
            return Err(format!("ledger line without `id | signature | reason`: {l}"));
        }
        if !f[1].starts_with("walker=") || !f[1].contains(" resolved=") || !f[1].contains(" compiled=") {
            return Err(format!("ledger signature is not `walker=A resolved=B compiled=C`: {l}"));
        }
        if !seen.insert(f[0].trim().to_string()) {
            return Err(format!("duplicate ledger id: {}", f[0]));
        }
    }
    Ok(())
}

#[test]
fn the_divergence_ledger_stays_within_its_ceiling() {
    let text = fs::read_to_string(root().join("tests/drift_corpus/known_divergences.txt")).unwrap();
    let ceiling: usize = fs::read_to_string(root().join("tests/drift_corpus/ledger_ceiling.txt")).unwrap().trim().parse().unwrap();
    if let Err(m) = check_ledger(&text, ceiling) {
        panic!("{m}");
    }
}

/// Self-test: the check notices each way a ledger can go wrong.
#[test]
fn the_ledger_check_notices_growth_and_malformed_lines() {
    let good = "# header\na | walker=x resolved=x compiled=y | why\nb | walker=x resolved=x compiled=y | why\n";
    assert!(check_ledger(good, 2).is_ok());
    assert!(check_ledger(good, 1).unwrap_err().contains("above its ceiling"), "growth must fail");
    assert!(check_ledger("a | walker=x resolved=x compiled=y |\n", 5).is_err(), "missing reason");
    assert!(check_ledger("a | something else | why\n", 5).is_err(), "bad signature");
    let dup = "a | walker=x resolved=x compiled=y | why\na | walker=x resolved=x compiled=z | why\n";
    assert!(check_ledger(dup, 5).unwrap_err().contains("duplicate"));
}

/// The contract document names the ledger and the gate it rests on.
#[test]
fn the_contract_points_at_the_ledger_and_the_gate() {
    let doc = fs::read_to_string(root().join("docs/testing/INTERPRETER_CONTRACT.md")).unwrap();
    for needle in ["tests/drift_corpus/known_divergences.txt", "scripts/wj/drift_gate.wj", "ledger_ceiling.txt"] {
        assert!(doc.contains(needle), "INTERPRETER_CONTRACT.md does not mention {needle}");
    }
}
