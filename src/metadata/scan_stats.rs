//! How much of the file system a build's metadata merge touched: directories listed and `.wj.meta` files read.
//!
//! `WJ_META_SCAN_STATS=1` prints the cumulative `meta-scan: dirs=N meta_files=M` to stderr after every metadata
//! root a command merges and when a build ends (`report_if_requested`); the last line is the command's total. Timing-independent evidence for the location-agnostic gate
//! (tests/meta_scan_locality.rs): a single-file build must touch the same amount whatever surrounds the file.

use std::sync::atomic::{AtomicUsize, Ordering};

static DIRS: AtomicUsize = AtomicUsize::new(0);
static META_FILES: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn dir_listed() {
    DIRS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn meta_file_read() {
    META_FILES.fetch_add(1, Ordering::Relaxed);
}

/// (directories listed, `.wj.meta` files read) since the process started.
pub fn counts() -> (usize, usize) {
    (DIRS.load(Ordering::Relaxed), META_FILES.load(Ordering::Relaxed))
}

/// Print the counts to stderr when `WJ_META_SCAN_STATS=1`.
pub fn report_if_requested() {
    if std::env::var_os("WJ_META_SCAN_STATS").is_some_and(|v| v == "1") {
        let (d, m) = counts();
        eprintln!("meta-scan: dirs={d} meta_files={m}");
    }
}
