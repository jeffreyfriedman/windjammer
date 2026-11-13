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

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ledger_lines(text: &str) -> Vec<&str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect()
}

/// The streams that own known divergences (the same list as `OWNERS` in scripts/wj/drift_ledger.wj).
const OWNERS: [&str; 9] = [
    "numeric-width",
    "codegen-refs",
    "codegen-types",
    "codegen-ownership",
    "runtime-surface",
    "parser-lexer",
    "language-decision",
    "gate-harness",
    "interp-literal-typing",
    "interp-check",
];

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
        if f.len() != 4 || f.iter().any(|x| x.trim().is_empty()) {
            return Err(format!("ledger line without `id | signature | reason | owner: STREAM`: {l}"));
        }
        let owner = f[3].trim().strip_prefix("owner: ").unwrap_or("").trim();
        if !OWNERS.contains(&owner) {
            return Err(format!("ledger line without a known owner stream ({}): {l}", OWNERS.join(", ")));
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
    let good = "# header\na | walker=x resolved=x compiled=y | why | owner: codegen-types\nb | walker=x resolved=x compiled=y | why | owner: numeric-width\n";
    assert!(check_ledger(good, 2).is_ok());
    assert!(check_ledger(good, 1).unwrap_err().contains("above its ceiling"), "growth must fail");
    assert!(check_ledger("a | walker=x resolved=x compiled=y | | owner: codegen-types\n", 5).is_err(), "missing reason");
    assert!(check_ledger("a | something else | why | owner: codegen-types\n", 5).is_err(), "bad signature");
    assert!(check_ledger("a | walker=x resolved=x compiled=y | why\n", 5).unwrap_err().contains("owner"), "no owner");
    assert!(check_ledger("a | walker=x resolved=x compiled=y | why | owner: nobody\n", 5).unwrap_err().contains("known owner"), "unknown owner");
    let dup = "a | walker=x resolved=x compiled=y | why | owner: codegen-types\na | walker=x resolved=x compiled=z | why | owner: codegen-types\n";
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

/// Repros named by the drift findings (`docs/drift/*.md`), as `tests/drift_corpus/repros/x.wj`, `pilotN-repros/x.wj`
/// or in a findings table's first column (`| `x` | ...`), and every file of a `docs/drift/*-repros/` directory.
fn drift_repro_names(docs: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(docs) else { return out };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let from = p.file_name().unwrap().to_string_lossy().to_string();
        if p.is_dir() && from.ends_with("-repros") {
            for f in fs::read_dir(&p).unwrap().flatten() {
                let n = f.file_name().to_string_lossy().to_string();
                if let Some(stem) = n.strip_suffix(".wj") {
                    out.push((stem.to_string(), format!("docs/drift/{from}/")));
                }
            }
        } else if from.ends_with(".md") {
            let text = fs::read_to_string(&p).unwrap();
            for line in text.lines() {
                // a closed finding names a repro that has left the corpus on purpose
                if line.contains("**Closed**") || line.contains("**Fixed**") {
                    continue;
                }
                for marker in ["tests/drift_corpus/repros/", "-repros/"] {
                    let mut rest = line;
                    while let Some(i) = rest.find(marker) {
                        rest = &rest[i + marker.len()..];
                        let stem: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                        if !stem.is_empty() && rest[stem.len()..].starts_with(".wj") {
                            out.push((stem, from.clone()));
                        }
                    }
                }
                // a findings table row: `| `name` | ...` (the repro's name)
                if let Some(cell) = line.strip_prefix("| `") {
                    let stem: String = cell.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    if !stem.is_empty() && cell[stem.len()..].starts_with("` |") && stem.contains('_') {
                        out.push((stem, from.clone()));
                    }
                }
            }
        }
    }
    out
}

/// Every repro the drift findings name runs in the drift corpus (`tests/drift_corpus/repros/`, or a value-position
/// program in `tests/engine_parity/`): a finding outside the corpus is a divergence no gate watches.
#[test]
fn every_drift_finding_repro_is_in_the_corpus() {
    // the corpus's programs: the repros, and the value-position programs it runs from tests/engine_parity
    let corpus = [root().join("tests/drift_corpus/repros"), root().join("tests/engine_parity")];
    let mut missing: Vec<String> = drift_repro_names(&root().join("docs/drift"))
        .into_iter()
        .filter(|(stem, _)| !corpus.iter().any(|d| d.join(format!("{stem}.wj")).is_file()))
        .map(|(stem, from)| format!("{stem} (named in {from})"))
        .collect();
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "drift repros outside tests/drift_corpus/repros: copy each in (and record or ledger it):\n{}", missing.join("\n"));
}

/// Self-test: the scan finds each way a finding names a repro, and skips closed findings.
#[test]
fn the_repro_scan_reads_paths_tables_and_repro_dirs() {
    let _scr = Scratch::new("drift-scan");
    let dir = _scr.path().to_path_buf();
    fs::create_dir_all(dir.join("pilot9-repros")).unwrap();
    fs::write(dir.join("pilot9-repros/in_dir.wj"), "").unwrap();
    fs::write(
        dir.join("f.md"),
        "see tests/drift_corpus/repros/by_path.wj\n| `table_row` | x |\n| `closed_one` | **Closed** |\n| `kind` | not a repro (no underscore) |\n",
    )
    .unwrap();
    let mut names: Vec<String> = drift_repro_names(&dir).into_iter().map(|(s, _)| s).collect();
    names.sort();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(names, ["by_path", "in_dir", "table_row"]);
}
