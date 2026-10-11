//! `wj check --watch`: poll modification times and re-check on change (results are cached).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Modification time of every `.wj` file under `path`.
pub fn snapshot(path: &Path) -> BTreeMap<PathBuf, SystemTime> {
    super::files::collect(path)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?;
            Some((p, m))
        })
        .collect()
}

/// Block until the snapshot differs from `before`; returns the new snapshot.
pub fn wait_for_change(path: &Path, before: &BTreeMap<PathBuf, SystemTime>, poll: Duration) -> BTreeMap<PathBuf, SystemTime> {
    loop {
        std::thread::sleep(poll);
        let now = snapshot(path);
        if &now != before {
            return now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_changes_when_a_file_is_edited_or_added() {
        let dir = std::env::temp_dir().join(format!("wj_check_watch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.wj"), "fn main() {}").unwrap();
        let s1 = snapshot(&dir);
        std::fs::write(dir.join("b.wj"), "fn x() {}").unwrap();
        let s2 = snapshot(&dir);
        assert_ne!(s1, s2);
        assert_eq!(s2.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
