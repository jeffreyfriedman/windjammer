//! Fast dev profile and binary publishing shared by `wj build` and `wj run --compiled`.
//!
//! Both commands compile in the same shared target tree with identical profile settings, so the
//! dependency artifacts (windjammer-runtime and friends) built by one are reused by the other.
//! Settings go through `CARGO_PROFILE_*` environment variables: the generated `Cargo.toml` is
//! byte-identical to before.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Dev profile tuned for turnaround: no debuginfo, incremental on (program crate), 256 codegen units.
pub fn apply_fast_dev_env(cmd: &mut Command) {
    cmd.env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_PROFILE_DEV_CODEGEN_UNITS", "256")
        .env("CARGO_PROFILE_DEV_LTO", "off")
        .env("CARGO_PROFILE_DEV_INCREMENTAL", "true");
}

/// Executable paths named by `cargo build --message-format=json*` output (compiler-artifact lines).
pub fn executables_from_cargo_json(stdout: &str) -> Vec<PathBuf> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["reason"] == "compiler-artifact")
        .filter_map(|v| v["executable"].as_str().map(PathBuf::from))
        .collect()
}

/// Copy built executables to `<output_dir>/target/<profile>/` (the documented location), skipping
/// copies whose bytes are already identical so unchanged builds touch nothing.
pub fn publish_executables(output_dir: &Path, release: bool, exes: &[PathBuf]) -> std::io::Result<()> {
    let dir = output_dir.join("target").join(if release { "release" } else { "debug" });
    std::fs::create_dir_all(&dir)?;
    for exe in exes {
        let Some(name) = exe.file_name() else { continue };
        let dest = dir.join(name);
        if dest.exists() && std::fs::read(&dest)? == std::fs::read(exe)? {
            continue;
        }
        let _ = std::fs::remove_file(&dest);
        std::fs::copy(exe, &dest)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_executables_only_from_artifact_lines() {
        let out = concat!(
            "{\"reason\":\"compiler-artifact\",\"executable\":null}\n",
            "{\"reason\":\"compiler-message\",\"executable\":\"/x/no\"}\n",
            "{\"reason\":\"compiler-artifact\",\"executable\":\"/x/yes\"}\n",
            "not json\n"
        );
        assert_eq!(executables_from_cargo_json(out), vec![PathBuf::from("/x/yes")]);
    }

    #[test]
    fn publish_copies_into_profile_dir_and_is_idempotent() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("prog");
        std::fs::write(&exe, b"v1").unwrap();
        publish_executables(t.path(), false, &[exe.clone()]).unwrap();
        let dest = t.path().join("target/debug/prog");
        assert_eq!(std::fs::read(&dest).unwrap(), b"v1");
        std::fs::write(&exe, b"v2").unwrap();
        publish_executables(t.path(), false, &[exe]).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"v2");
    }
}

/// Stable per-source work directory for `wj run --compiled` (`<cache>/run/<hash of canonical path>`).
/// A fixed path keeps the program crate's cargo fingerprint, so re-running unchanged source
/// recompiles nothing and edited source recompiles incrementally. Generated files are only
/// rewritten when their content changes.
pub fn run_workdir(source: &Path) -> PathBuf {
    let canon = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let mut h: u64 = 0xcbf29ce484222325;
    for b in canon.to_string_lossy().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let dir = super::cache_root().join("run").join(format!("{h:016x}"));
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(".used"), b""); // refreshes mtime for age-based pruning
    dir
}

#[cfg(test)]
mod workdir_tests {
    use super::*;

    #[test]
    fn workdir_is_stable_per_source_and_distinct_across_sources() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = (t.path().join("a.wj"), t.path().join("b.wj"));
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        assert_eq!(run_workdir(&a), run_workdir(&a));
        assert_ne!(run_workdir(&a), run_workdir(&b));
    }
}

/// Remove `run/<hash>` work directories whose `.used` stamp is older than `max_age`; returns bytes freed.
pub fn prune_run_workdirs(root: &Path, max_age: std::time::Duration) -> u64 {
    let mut freed = 0;
    let Ok(rd) = std::fs::read_dir(root.join("run")) else { return 0 };
    for e in rd.flatten() {
        let age = std::fs::metadata(e.path().join(".used"))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok());
        if age.is_some_and(|a| a > max_age) {
            let size = super::dir_size(&e.path());
            if std::fs::remove_dir_all(e.path()).is_ok() {
                freed += size;
            }
        }
    }
    freed
}
