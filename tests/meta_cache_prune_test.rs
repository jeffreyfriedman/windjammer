//! `wj cache prune` bounds the manifest-less metadata cache (`<cache root>/meta`, one directory per tree content key):
//! trees whose newest file is past the age limit go, the oldest beyond the tree cap go, and a tree written within the
//! hour stays (its writer may still be running) unless forced.

use std::path::Path;
use std::time::{Duration, SystemTime};
use windjammer::cargo_cache::entries::{prune_trees, MAX_AGE};
use windjammer::cargo_cache::PruneReport;

fn tree(dir: &Path, key: &str, age: Duration) {
    let t = dir.join(key).join("sub");
    std::fs::create_dir_all(&t).unwrap();
    let f = t.join("m.wj.meta");
    std::fs::write(&f, "{}").unwrap();
    let file = std::fs::File::options().write(true).open(&f).unwrap();
    file.set_modified(SystemTime::now() - age).unwrap();
}

fn report() -> PruneReport {
    PruneReport { freed_bytes: 0, removed_paths: Vec::new(), low_disk: false, messages: Vec::new() }
}

fn left(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
}

#[test]
fn prune_drops_old_trees_and_the_oldest_beyond_the_cap() {
    let d = tempfile::tempdir().unwrap();
    let meta = d.path().join("meta");
    let day = Duration::from_secs(24 * 60 * 60);
    tree(&meta, "expired", MAX_AGE + day);
    tree(&meta, "older", 3 * day);
    tree(&meta, "old", 2 * day);
    tree(&meta, "fresh", Duration::from_secs(60));
    let mut r = report();
    let freed = prune_trees(&meta, 2, false, &mut r);
    // expired by age; "older" is the oldest beyond the cap of 2; "fresh" is recent and protected regardless
    assert_eq!(left(&meta), ["fresh", "old"]);
    assert!(freed > 0 && r.removed_paths.len() == 2, "{r:?}");
}

#[test]
fn a_recent_tree_survives_the_cap_unless_forced() {
    let d = tempfile::tempdir().unwrap();
    let meta = d.path().join("meta");
    tree(&meta, "a", Duration::from_secs(120));
    tree(&meta, "b", Duration::from_secs(60));
    prune_trees(&meta, 1, false, &mut report());
    assert_eq!(left(&meta), ["a", "b"], "a tree written within the hour was pruned");
    prune_trees(&meta, 1, true, &mut report());
    assert_eq!(left(&meta), ["b"], "forced prune keeps only the newest tree");
}

#[test]
fn the_pruned_directory_is_where_the_compiler_writes() {
    let root = Path::new("/some/cache/root");
    assert_eq!(windjammer::metadata::global_meta_cache_dir(root), root.join("meta"));
}
