//! Cross-checkout analysis cache: the per-file analysis phase (`project::analysis_phase`) keyed on content alone.
//!
//! The phase reads one parsed file and the stdlib (see `file_cache.rs`), so a result is a function of the file's
//! text, the compiler build and the ambient `.wj.meta` sidecars. The key hashes exactly that (the analysis reads no
//! environment variable that changes a result) and names no path: the same file in two worktrees, or two checkouts of one commit, share an
//! entry. The only path-dependent part of a result is the file's own name inside its diagnostics; entries store
//! it as a placeholder and restore it on a hit, and a result whose text could confuse the two is not stored.
//!
//! The store itself (directory, locking, eviction) is injected by the driver (`cli::check`), so this module
//! knows no cache location. Nothing here can change what is reported: any doubt (unreadable entry, wrong
//! format, unknown build) is a miss and the analysis runs.

use crate::diagnostics::Diagnostic;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

const FORMAT: u32 = 1;
const PLACEHOLDER: &str = "@@wj-analysis-file@@";

/// Where entries live: opaque text under a hex key. Implementations must make `put` atomic for readers.
pub trait SharedStore: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn put(&self, key: &str, text: &str);
}

#[derive(Clone)]
struct Config {
    store: Arc<dyn SharedStore>,
    compiler: String,
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

/// Identity of this compiler build by content: version plus the build script's commit / uncommitted-diff / profile
/// key. No executable path or time. `None` for a build of unknown origin (no git): such a build shares nothing.
pub fn compiler_key() -> Option<String> {
    let key = option_env!("WJ_BUILD_KEY")?;
    (!key.starts_with("unknown")).then(|| format!("{}+{key}", env!("CARGO_PKG_VERSION")))
}

/// Use `store` for the analysis cache, keyed under this build. Returns false (nothing shared) for an unknown build.
pub fn install(store: Arc<dyn SharedStore>) -> bool {
    match compiler_key() {
        Some(compiler) => {
            install_for(store, compiler);
            true
        }
        None => false,
    }
}

/// [`install`] with an explicit compiler key (tests exercise invalidation by changing it).
pub fn install_for(store: Arc<dyn SharedStore>, compiler: String) {
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = Some(Config { store, compiler });
}

pub fn uninstall() {
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// A name part unique to this process, thread and moment, for temporary files two writers must not share.
pub fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    format!("{}-{:?}-{nanos}", std::process::id(), std::thread::current().id()).replace(['(', ')'], "")
}

/// SHA-256 over the format, compiler key, ambient digest and the file text, length-prefixed.
pub fn key(compiler: &str, ambient: &str, text: &str) -> String {
    let mut h = Sha256::new();
    let mut part = |b: &[u8]| {
        h.update((b.len() as u64).to_le_bytes());
        h.update(b);
    };
    part(b"wj-analysis");
    part(&FORMAT.to_le_bytes());
    part(compiler.as_bytes());
    part(ambient.as_bytes());
    part(text.as_bytes());
    format!("{:x}", h.finalize())
}

/// A lookup handle for one check run; `None` when no store is installed or the run is uncacheable.
pub struct Shared {
    store: Arc<dyn SharedStore>,
    prefix_compiler: String,
    ambient: String,
}

impl Shared {
    pub fn current(ambient_portable: Option<String>) -> Option<Self> {
        let cfg = CONFIG.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
        Some(Self { store: cfg.store, prefix_compiler: cfg.compiler, ambient: ambient_portable? })
    }

    fn key(&self, text: &str) -> String {
        key(&self.prefix_compiler, &self.ambient, text)
    }

    /// The stored analysis of `text`, with diagnostics naming `file`.
    pub fn get(&self, file: &str, text: &str) -> Option<Vec<Diagnostic>> {
        let found = self.store.get(&self.key(text)).and_then(|s| restore(&s, file));
        found
    }

    pub fn put(&self, file: &str, text: &str, diags: &[Diagnostic]) {
        if let Some(s) = detach(file, text, diags) {
            self.store.put(&self.key(text), &s);
        }
    }
}

fn escaped(file: &str) -> String {
    let q = serde_json::to_string(file).unwrap_or_default();
    q.trim_matches('"').to_string()
}

/// Diagnostics as text with `file` replaced by a placeholder; `None` when that cannot be undone exactly (the
/// source mentions the file's name, or the placeholder itself appears).
fn detach(file: &str, text: &str, diags: &[Diagnostic]) -> Option<String> {
    let esc = escaped(file);
    if file.is_empty() || text.contains(file) || text.contains(PLACEHOLDER) || file.contains(PLACEHOLDER) {
        return None;
    }
    let json = serde_json::to_string(&(FORMAT, diags)).ok()?;
    Some(json.replace(&esc, PLACEHOLDER))
}

fn restore(stored: &str, file: &str) -> Option<Vec<Diagnostic>> {
    let json = stored.replace(PLACEHOLDER, &escaped(file));
    let (format, diags): (u32, Vec<Diagnostic>) = serde_json::from_str(&json).ok()?;
    (format == FORMAT).then_some(diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(file: &str) -> Diagnostic {
        let mut d = Diagnostic::error("WJ0027", format!("note in {file}")).with_note(format!("in {file}"));
        d.primary_span = Some(crate::diagnostics::Span { file: file.to_string(), byte_start: 0, byte_end: 1, line_start: 1, col_start: 1, line_end: 1, col_end: 2, label: None });
        d
    }

    #[test]
    fn file_name_is_rebased_exactly() {
        let stored = detach("/a/b \"q\"/main.wj", "fn main() {}", &[diag("/a/b \"q\"/main.wj")]).expect("storable");
        assert!(!stored.contains("/a/b"));
        let back = restore(&stored, "/other/place/main.wj").expect("restores");
        assert_eq!(back, vec![diag("/other/place/main.wj")]);
    }

    #[test]
    fn ambiguous_results_are_not_stored() {
        assert!(detach("main.wj", "// see main.wj", &[]).is_none(), "source mentions the file name");
        assert!(detach("main.wj", PLACEHOLDER, &[]).is_none());
    }
}
