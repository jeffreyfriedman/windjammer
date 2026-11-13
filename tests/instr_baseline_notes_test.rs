//! Rows of the instruction baseline (`bench/interp/instr_baseline.json`) raised for a known cost carry a note
//! (`notes.<counter>.<kernel>`: `raised_by`, `lower_when`). A note whose condition has come true fails here, so a
//! raised row cannot outlive the change that was to lower it.
//! Run: cargo test --release --test all -- instr_baseline_notes

use std::path::Path;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn baseline() -> serde_json::Value {
    let text = std::fs::read_to_string(Path::new(ROOT).join("bench/interp/instr_baseline.json")).expect("read the baseline");
    serde_json::from_str(&text).expect("the baseline is JSON")
}

/// `(counter, kernel, lower_when)` of every note.
fn notes(doc: &serde_json::Value) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for (counter, rows) in doc.get("notes").and_then(|n| n.as_object()).into_iter().flatten() {
        for (kernel, note) in rows.as_object().into_iter().flatten() {
            let when = note.get("lower_when").and_then(|w| w.as_str()).unwrap_or_default().to_string();
            out.push((counter.clone(), kernel.clone(), when));
        }
    }
    out
}

/// Every note names a row that exists, with its reason and its condition.
#[test]
fn every_note_names_a_recorded_row_and_says_why() {
    let doc = baseline();
    for (counter, kernel, when) in notes(&doc) {
        assert!(doc.get(&counter).and_then(|t| t.get(&kernel)).is_some(), "note for {counter}/{kernel}: no such row");
        let note = &doc["notes"][&counter][&kernel];
        assert!(note.get("raised_by").and_then(|r| r.as_str()).is_some_and(|r| !r.is_empty()), "note for {counter}/{kernel}: no raised_by");
        assert!(!when.is_empty(), "note for {counter}/{kernel}: no lower_when");
    }
}

/// The rows raised for the run-time literal widths (ADR-0038 C6b) are to be lowered by the check-result cache
/// (`src/check/run_cache.rs`, branch interp-cache): once it is in the tree, the note must be gone.
#[test]
fn rows_raised_until_interp_cache_are_lowered_once_it_is_merged() {
    let merged = Path::new(ROOT).join("src/check/run_cache.rs").exists();
    let pending: Vec<String> = notes(&baseline())
        .into_iter()
        .filter(|(_, _, when)| when.contains("src/check/run_cache.rs"))
        .map(|(c, k, _)| format!("{c}/{k}"))
        .collect();
    assert!(
        !merged || pending.is_empty(),
        "interp-cache is merged: re-run interp_instr --write and remove the raised_by note ({})",
        pending.join(", ")
    );
}
