//! Rerun-stability helpers shared by `build.rs` and `tests/build_rerun_stability_test.rs` (via `#[path]`).
//!
//! The crate recompiles (minutes) when a file its sources `include!` gets a newer mtime, and a build script
//! rerun is triggered by any mtime change under its `rerun-if-changed` paths, including `.git/index`, which a
//! plain `git status` rewrites. Two rules keep a rerun cheap:
//!   * generated files are written only when their content changes (`write_if_changed`);
//!   * git is asked read-only questions (`git_command` adds `--no-optional-locks`, so `status` never refreshes
//!     the index; use `diff-index -p HEAD`, because `git diff HEAD` rewrites a stale index even with that flag).

use std::path::Path;
use std::process::Command;

/// Write `content` to `path` unless the file already holds exactly that. Returns whether it wrote.
pub fn write_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    if std::fs::read(path).is_ok_and(|existing| existing == content.as_bytes()) {
        return Ok(false);
    }
    std::fs::write(path, content)?;
    Ok(true)
}

/// A `git` invocation in `dir` that never takes optional locks (does not rewrite `.git/index`).
pub fn git_command(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("--no-optional-locks").args(args).current_dir(dir);
    cmd
}
