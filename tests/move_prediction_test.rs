//! Uses after a move predicted from the generated Rust (`check::moves`, WJ0007 before an interpreted run).
//!
//! Precision gate: across the repository's single-file corpora, every prediction must be a program rustc
//! rejects with E0382; the list of files with a prediction is pinned here and each entry was checked
//! against the compiled build. A new prediction fails until it is confirmed and added (or fixed).

use std::path::{Path, PathBuf};

fn wj_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            wj_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p);
        }
    }
}

/// Every WJ0007 shape points at the move: a primary span on the line that consumes the value, and, for a
/// move inside a loop, a secondary span on the loop header.
#[test]
fn every_predicted_move_has_a_span_and_loop_moves_name_the_loop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/move_prediction");
    let mut files = Vec::new();
    wj_files(&root, &mut files);
    files.retain(|f| !f.to_string_lossy().contains("/accepted/"));
    files.sort();
    let mut n = 0;
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        for d in windjammer::check::moves::predicted_moves(f) {
            n += 1;
            let s = d.primary_span.as_ref().unwrap_or_else(|| panic!("{}: no span: {}", f.display(), d.message));
            let line = src.lines().nth(s.line_start - 1).unwrap_or("");
            let var = d.message.split('`').nth(1).unwrap_or("");
            assert!(line.contains(var), "{}: span line {} does not mention `{var}`: {line}", f.display(), s.line_start);
            // a `let` move (alias or field) points at that `let`, not at a read of the same text elsewhere
            if let Some(l) = d.message.split('`').find(|p| p.starts_with("let ")) {
                let head = l.split(" =").next().unwrap_or(l);
                assert!(line.contains(head), "{}: span line {} is not `{head}`: {line}", f.display(), s.line_start);
            }
            if d.message.contains("every iteration of the loop") {
                let l = d.secondary_spans.first().unwrap_or_else(|| panic!("{}: loop move without the loop: {}", f.display(), d.message));
                let header = src.lines().nth(l.line_start - 1).unwrap_or("").trim_start();
                assert!(["for ", "while ", "loop"].iter().any(|k| header.starts_with(k)), "{}: {header}", f.display());
            }
        }
    }
    assert!(n >= 7, "only {n} predictions");
}

/// Values that cannot be copied (they hold a channel end) are the moves WJ0007 still reports, one per
/// kind: an owned receiver, an owning argument, a `let` alias, a move inside a loop, a partial move.
#[test]
fn moves_of_values_that_cannot_be_copied_are_reported_per_kind() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/move_prediction");
    let cases = [
        ("receiver_non_clone.wj", "`inbox` is used after `inbox.with_seen()` consumed it: `Inbox::with_seen` takes ownership of its receiver"),
        ("argument_non_clone.wj", "`inbox` is used after the call to `drain` consumed it (its parameter takes ownership)"),
        ("let_alias_non_clone.wj", "`inbox` is used after `let other = inbox` consumed it"),
        ("loop_non_clone.wj", "`inbox` is moved by the call to `drain` in every iteration of the loop"),
        ("loop_if_arm_non_clone.wj", "`inbox` is moved by `let arm = inbox` in every iteration of the loop"),
        ("partial_non_clone.wj", "its field `chan` is gone"),
    ];
    for (f, want) in cases {
        let d = windjammer::check::moves::predicted_moves(&root.join(f));
        assert_eq!(d.len(), 1, "{f}: {d:?}");
        assert_eq!(d[0].code, "WJ0007", "{f}");
        assert!(d[0].message.contains(want), "{f}: {}", d[0].message);
        let help = d[0].help.join(" ");
        assert!(!help.contains(".clone()"), "{f}: a copy is impossible, the help must not offer one: {help}");
    }
}

/// The compiler clones where a moved value is used again; those programs build and nothing is reported.
#[test]
fn moves_the_compiler_turns_into_clones_are_not_reported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/move_prediction/accepted");
    // C6: the moves WJ0007 used to predict (receiver by value, owning argument, inside a loop) are
    // now copies the compiler decides from liveness, so these programs build and nothing is reported.
    for f in [
        "let_copy_auto_cloned.wj",
        "field_auto_cloned.wj",
        "arg_move_reuse.wj",
        "arg_move_in_loop.wj",
        "self_by_value_reuse.wj",
    ] {
        let d = windjammer::check::moves::predicted_moves(&root.join(f));
        assert!(d.is_empty(), "{f}: {d:?}");
    }
}

/// A value without `Clone` (a struct holding a trait object or a channel end) is never copied: reading it
/// again after a move is reported as WJ0007.
#[test]
fn moved_values_without_clone_are_still_reported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/move_prediction/rejected");
    let d = windjammer::check::moves::predicted_moves(&root.join("box_dyn_holder_reuse.wj"));
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].code, "WJ0007");
    assert!(d[0].message.contains("`h`"), "{}", d[0].message);
}

#[test]
fn predictions_across_the_corpora() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for d in ["examples", "tests"] {
        wj_files(&root.join(d), &mut files);
    }
    files.sort();
    // The drift corpus is a copy of whole multi-file projects (their types live in sibling files): one file of it read
    // alone cannot be judged, and the drift corpus has its own compiled baseline.
    files.retain(|f| !f.starts_with(root.join("tests/drift_corpus")));
    let mut hits = Vec::new();
    for f in &files {
        let d = windjammer::check::moves::predicted_moves(f);
        if !d.is_empty() {
            hits.push(format!("{} {}", f.strip_prefix(root).unwrap_or(f).display(), d.len()));
        }
    }
    // each confirmed with rustc on the generated code (E0382). Since C6 the compiler copies a moved
    // value that is read again (`string_literal_move_then_use`, `test_ownership` and the former WJ0007
    // fixtures now build), so no single-file corpus program is predicted to fail.
    let expected: &[&str] = &[
        "tests/move_prediction/argument_non_clone.wj 1",
        "tests/move_prediction/let_alias_non_clone.wj 1",
        "tests/move_prediction/loop_if_arm_non_clone.wj 1",
        "tests/move_prediction/loop_non_clone.wj 1",
        "tests/move_prediction/partial_non_clone.wj 1",
        "tests/move_prediction/receiver_non_clone.wj 1",
        "tests/move_prediction/rejected/box_dyn_holder_reuse.wj 1",
    ];
    assert_eq!(hits, expected, "{} files scanned", files.len());
}

/// Real agent programs: the Windjammer drafts of agent pilot 4 (first and final source of every trial). A
/// prediction on one must be confirmed against the compiled build and pinned here.
#[test]
fn predictions_on_agent_drafts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut trials = Vec::new();
    wj_json(&root.join("bench/agent-dx/results/pilot4/trials"), &mut trials);
    trials.sort();
    let dir = std::env::temp_dir().join(format!("wj-move-drafts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut hits = Vec::new();
    let mut n = 0;
    for t in &trials {
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(t).unwrap()).unwrap();
        if doc["lang"] != "wj" {
            continue;
        }
        for key in ["first_source", "final_source"] {
            let Some(src) = doc[key].as_str().filter(|s| !s.is_empty()) else { continue };
            let f = dir.join("main.wj");
            std::fs::write(&f, src).unwrap();
            n += 1;
            if !windjammer::check::moves::predicted_moves(&f).is_empty() {
                hits.push(format!("{} {key}", t.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    let expected: &[&str] = &[];
    assert!(n > 100, "only {n} drafts");
    assert_eq!(hits, expected, "{n} drafts scanned");
}

fn wj_json(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if e.path().extension().is_some_and(|x| x == "json") {
            out.push(e.path());
        }
    }
}
