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
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/drift_corpus/repros/self_by_value_reuse.wj");
    let d = windjammer::check::moves::predicted_moves(&p);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].code, "WJ0007");
    assert!(d[0].message.contains("`n`") && d[0].message.contains("Node::advance"), "{}", d[0].message);
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
    let expected: &[&str] = &["tests/drift_corpus/repros/self_by_value_reuse.wj 1"];
    assert_eq!(hits, expected, "{} files scanned", files.len());
}
