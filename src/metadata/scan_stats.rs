//! How much of the file system a build's metadata merge touched: directories listed and `.wj.meta` files read.
//!
//! `WJ_META_SCAN_STATS=1` prints one `meta-scan: dir` line per directory listed and one `meta-scan: file` line per
//! `.wj.meta` read, to stderr; a consumer counts the lines (no state is kept here). Timing-independent evidence for
//! the location-agnostic gate (tests/meta_scan_locality.rs): a single-file build must touch the same amount whatever
//! surrounds the file.

fn event(kind: &str) {
    if crate::debug_env::equals("WJ_META_SCAN_STATS", "1") {
        eprintln!("meta-scan: {kind}");
    }
}

pub(crate) fn dir_listed() {
    event("dir");
}

pub(crate) fn meta_file_read() {
    event("file");
}

/// One closure file of a manifest-less program read for its cache key (`projectless_key`).
pub(crate) fn closure_file() {
    event("closure");
}

/// One `.wj.meta` read for the ambient run-cache fingerprint (`check::ambient`).
pub(crate) fn ambient_file() {
    event("ambient");
}
