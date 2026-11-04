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

#[test]
fn self_by_value_receiver_used_again_is_reported() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/diagnostics_oracle/check/wj0007_self_by_value_reuse.wj");
    let d = windjammer::check::moves::predicted_moves(&p);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].code, "WJ0007");
    assert!(d[0].message.contains("`n`") && d[0].message.contains("Node::advance"), "{}", d[0].message);
}

/// Class b, and c (the same move inside a loop), each rejected by rustc with E0382 on the generated code.
#[test]
fn moves_into_owning_parameters_are_reported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/diagnostics_oracle/check");
    let d = windjammer::check::moves::predicted_moves(&root.join("wj0007_arg_move_reuse.wj"));
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].message.contains("`xs` is used after the call to `wrap` moved it"), "{}", d[0].message);
    let d = windjammer::check::moves::predicted_moves(&root.join("wj0007_arg_move_in_loop.wj"));
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].message.contains("in every iteration of the loop"), "{}", d[0].message);
}

/// The compiler clones where a moved value is used again; those programs build and nothing is reported.
#[test]
fn moves_the_compiler_turns_into_clones_are_not_reported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/move_prediction/accepted");
    for f in ["let_copy_auto_cloned.wj", "field_auto_cloned.wj"] {
        let d = windjammer::check::moves::predicted_moves(&root.join(f));
        assert!(d.is_empty(), "{f}: {d:?}");
    }
}

#[test]
fn predictions_across_the_corpora() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for d in ["examples", "tests"] {
        wj_files(&root.join(d), &mut files);
    }
    files.sort();
    let mut hits = Vec::new();
    for f in &files {
        let d = windjammer::check::moves::predicted_moves(f);
        if !d.is_empty() {
            hits.push(format!("{} {}", f.strip_prefix(root).unwrap_or(f).display(), d.len()));
        }
    }
    // each confirmed with rustc on the generated code (E0382): `test_ownership` is an ownership-error
    // fixture, `string_literal_move_then_use` is the compiler defect D-CLONE-STRLIT (no clone where the
    // interpreter's copy is expected)
    let expected: &[&str] = &[
        "tests/diagnostics_oracle/check/wj0007_arg_move_in_loop.wj 1",
        "tests/diagnostics_oracle/check/wj0007_arg_move_reuse.wj 1",
        "tests/diagnostics_oracle/check/wj0007_self_by_value_reuse.wj 1",
        "tests/error_system/test_ownership.wj 1",
        "tests/inference_corpus/string_literal_move_then_use.wj 1",
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
