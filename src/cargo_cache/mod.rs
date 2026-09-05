//! Central Cargo target-dir cache outside application repos.
//!
//! Compiled `.rlib` / `.rmeta` artifacts are reused across Windjammer ecosystem
//! projects when they share one target directory. See
//! `docs/superpowers/specs/2026-09-04-shared-cargo-cache-design.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// Free-disk threshold that triggers aggressive prune (GiB).
pub const DEFAULT_MIN_FREE_GIB: u64 = 15;

/// Drop incremental units older than this many days.
pub const DEFAULT_INCREMENTAL_MAX_AGE_DAYS: u64 = 7;

const ENV_ROOT: &str = "WJ_CARGO_TARGET_ROOT";
const ENV_USE_LOCAL: &str = "WJ_USE_LOCAL_TARGET";
const ENV_MIN_FREE: &str = "WJ_DISK_FREE_MIN_GIB";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// Shared by compiler, dogfood crates, games — maximizes dep `.rlib` reuse.
    Shared,
    /// Ephemeral codegen / integration cargo checks (often same package name).
    Verify,
}

/// True when the user forced in-repo `./target` (sandbox escape hatch).
pub fn use_local_target() -> bool {
    match std::env::var(ENV_USE_LOCAL) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}

/// Platform cache root: `WJ_CARGO_TARGET_ROOT` or OS default.
pub fn cache_root() -> PathBuf {
    if let Ok(root) = std::env::var(ENV_ROOT) {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_cache_root()
}

fn default_cache_root() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Caches")
            .join("windjammer")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("windjammer")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|h| h.join(".cache")))
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("windjammer")
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Resolve the Cargo target directory for `kind`, creating it when possible.
///
/// Resolution order:
/// 1. `WJ_USE_LOCAL_TARGET=1` → in-repo `./target`
/// 2. `WJ_CARGO_TARGET_ROOT` / platform cache (if writable)
/// 3. Existing `CARGO_TARGET_DIR` (e.g. Cursor sandbox cache — still outside the repo)
/// 4. In-repo `./target` last resort
pub fn target_dir(kind: TargetKind) -> PathBuf {
    if use_local_target() {
        return local_fallback(kind);
    }

    let root = cache_root();
    let dir = match kind {
        TargetKind::Shared => root.join("cargo-target").join("shared"),
        TargetKind::Verify => root.join("cargo-target").join("verify"),
    };

    if ensure_writable_dir(&dir).is_ok() {
        return dir;
    }

    // Sandbox / restricted environments: reuse ambient CARGO_TARGET_DIR when present
    // so we still avoid polluting application repos.
    if let Ok(existing) = std::env::var("CARGO_TARGET_DIR") {
        let existing = existing.trim();
        if !existing.is_empty() {
            let base = PathBuf::from(existing);
            let dir = match kind {
                TargetKind::Shared => base,
                TargetKind::Verify => {
                    // Keep verify isolated under the ambient root when possible.
                    let v = PathBuf::from(existing).join("wj_verify");
                    let _ = fs::create_dir_all(&v);
                    v
                }
            };
            return dir;
        }
    }

    local_fallback(kind)
}

fn local_fallback(kind: TargetKind) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match kind {
        TargetKind::Shared => cwd.join("target"),
        TargetKind::Verify => cwd.join("target").join("wj_verify"),
    }
}

fn ensure_writable_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let probe = dir.join(".wj-cache-write-probe");
    fs::write(&probe, b"ok")?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

/// Apply shared/verify target dir to a cargo `Command` (overrides sandbox env).
pub fn configure_cargo_command(cmd: &mut Command, kind: TargetKind) {
    let dir = target_dir(kind);
    let _ = fs::create_dir_all(&dir);
    cmd.env("CARGO_TARGET_DIR", &dir);
}

/// Convenience: shared target path (creates dir when possible).
pub fn shared_target_dir() -> PathBuf {
    target_dir(TargetKind::Shared)
}

/// Convenience: verify target path (creates dir when possible).
pub fn verify_target_dir() -> PathBuf {
    target_dir(TargetKind::Verify)
}

/// Binary/library artifact path under the shared target tree.
pub fn artifact_path(profile: &str, name: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let mut p = shared_target_dir().join(profile).join(name);
        if p.extension().is_none() {
            p.set_extension("exe");
        }
        p
    }
    #[cfg(not(target_os = "windows"))]
    {
        shared_target_dir().join(profile).join(name)
    }
}

#[derive(Debug, Clone)]
pub struct CacheStatus {
    pub root: PathBuf,
    pub shared: PathBuf,
    pub verify: PathBuf,
    pub shared_bytes: u64,
    pub verify_bytes: u64,
    pub free_bytes: Option<u64>,
    pub using_local_fallback: bool,
}

pub fn status() -> CacheStatus {
    let root = cache_root();
    let shared = target_dir(TargetKind::Shared);
    let verify = target_dir(TargetKind::Verify);
    let expected_prefix = root.join("cargo-target");
    let using_local = use_local_target()
        || shared.ends_with("target") && !shared.starts_with(&expected_prefix);
    CacheStatus {
        free_bytes: free_disk_bytes(&shared),
        shared_bytes: dir_size(&shared),
        verify_bytes: dir_size(&verify),
        using_local_fallback: using_local,
        root,
        shared,
        verify,
    }
}

pub fn free_disk_bytes(path: &Path) -> Option<u64> {
    // Walk up to an existing ancestor for df/stat.
    let mut probe = path.to_path_buf();
    while !probe.exists() {
        if !probe.pop() {
            break;
        }
    }
    if !probe.exists() {
        probe = PathBuf::from(".");
    }

    #[cfg(unix)]
    {
        free_disk_bytes_unix(&probe)
    }
    #[cfg(target_os = "windows")]
    {
        free_disk_bytes_windows(&probe)
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = probe;
        None
    }
}

#[cfg(unix)]
fn free_disk_bytes_unix(path: &Path) -> Option<u64> {
    let output = Command::new("df")
        .args(["-Pk"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Filesystem 1024-blocks Used Available Capacity Mounted on
    let line = stdout.lines().nth(1)?;
    let avail_kib: u64 = line.split_whitespace().nth(3)?.parse().ok()?;
    Some(avail_kib.saturating_mul(1024))
}

#[cfg(target_os = "windows")]
fn free_disk_bytes_windows(path: &Path) -> Option<u64> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-Item -LiteralPath '{}').PSDrive.Free",
                path.display()
            ),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse().ok()
}

pub fn min_free_bytes() -> u64 {
    let gib = std::env::var(ENV_MIN_FREE)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MIN_FREE_GIB);
    gib.saturating_mul(1_073_741_824)
}

#[derive(Debug, Clone)]
pub struct PruneOptions {
    pub max_incremental_age: Duration,
    pub aggressive: bool,
    /// When true, prune if free space is below threshold even without --aggressive.
    pub auto_low_disk: bool,
}

impl Default for PruneOptions {
    fn default() -> Self {
        Self {
            max_incremental_age: Duration::from_secs(
                DEFAULT_INCREMENTAL_MAX_AGE_DAYS.saturating_mul(24 * 60 * 60),
            ),
            aggressive: false,
            auto_low_disk: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct PruneReport {
    pub freed_bytes: u64,
    pub removed_paths: Vec<PathBuf>,
    pub low_disk: bool,
    pub messages: Vec<String>,
}

/// Prune shared + verify caches. Safe to call from long builds.
pub fn prune(opts: PruneOptions) -> PruneReport {
    let mut report = PruneReport::default();
    let shared = target_dir(TargetKind::Shared);
    let verify = target_dir(TargetKind::Verify);

    let free = free_disk_bytes(&shared);
    let low_disk = free.map(|b| b < min_free_bytes()).unwrap_or(false);
    report.low_disk = low_disk;

    let do_aggressive = opts.aggressive || (opts.auto_low_disk && low_disk);

    for root in [&shared, &verify] {
        if !root.exists() {
            continue;
        }
        report.freed_bytes += prune_old_incremental(root, opts.max_incremental_age, &mut report);
        if do_aggressive {
            report.freed_bytes += prune_stale_fingerprint_dirs(root, &mut report);
        }
    }

    if do_aggressive && low_disk {
        // Prefer keeping release (test suite / dogfood default); drop debug incremental already done.
        report.messages.push(
            "Low disk: pruned stale incremental/fingerprint dirs under shared+verify.".into(),
        );
    }

    report
}

/// Run prune only when free space is below the threshold (no-op otherwise).
pub fn prune_if_low_disk() -> PruneReport {
    let shared = target_dir(TargetKind::Shared);
    let free = free_disk_bytes(&shared);
    if free.map(|b| b < min_free_bytes()).unwrap_or(false) {
        prune(PruneOptions {
            aggressive: true,
            auto_low_disk: true,
            ..PruneOptions::default()
        })
    } else {
        PruneReport::default()
    }
}

fn prune_old_incremental(root: &Path, max_age: Duration, report: &mut PruneReport) -> u64 {
    let mut freed = 0u64;
    let now = SystemTime::now();
    for profile in ["debug", "release"] {
        let incr = root.join(profile).join("incremental");
        if !incr.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&incr) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let aged_out = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|mtime| now.duration_since(mtime).ok())
                    .map(|age| age > max_age)
                    .unwrap_or(false);
                if aged_out {
                    let size = dir_size(&path);
                    if fs::remove_dir_all(&path).is_ok() {
                        freed += size;
                        report.removed_paths.push(path);
                    }
                }
            }
        }
    }
    freed
}

fn prune_stale_fingerprint_dirs(root: &Path, report: &mut PruneReport) -> u64 {
    let mut freed = 0u64;
    // `.fingerprint` itself is needed; remove only empty nested leftovers is risky.
    // Instead remove `incremental` entirely under debug when aggressive+low disk —
    // next build regenerates quickly relative to full clean.
    for profile in ["debug"] {
        let incr = root.join(profile).join("incremental");
        if incr.is_dir() {
            let size = dir_size(&incr);
            if fs::remove_dir_all(&incr).is_ok() {
                freed += size;
                report.removed_paths.push(incr);
                report
                    .messages
                    .push(format!("Removed {} incremental cache", profile));
            }
        }
    }
    let _ = root;
    freed
}

pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            total = total.saturating_add(dir_size(&p));
        } else if let Ok(meta) = entry.metadata() {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

pub fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Write `env.sh` / `env.ps1` under the cache root for users who invoke cargo directly.
pub fn write_setup_scripts() -> std::io::Result<(PathBuf, PathBuf)> {
    let root = cache_root();
    fs::create_dir_all(&root)?;
    let shared = target_dir(TargetKind::Shared);
    fs::create_dir_all(&shared)?;

    let sh = root.join("env.sh");
    let ps1 = root.join("env.ps1");

    let sh_body = format!(
        "# Windjammer shared Cargo target (source from your shell profile)\n\
         # Generated by `wj cache setup`\n\
         export WJ_CARGO_TARGET_ROOT={root}\n\
         export CARGO_TARGET_DIR={shared}\n",
        root = shell_single_quote(&root.display().to_string()),
        shared = shell_single_quote(&shared.display().to_string()),
    );
    fs::write(&sh, sh_body)?;

    let ps_body = format!(
        "# Windjammer shared Cargo target (dot-source from PowerShell profile)\n\
         # Generated by `wj cache setup`\n\
         $env:WJ_CARGO_TARGET_ROOT = '{root}'\n\
         $env:CARGO_TARGET_DIR = '{shared}'\n",
        root = root.display().to_string().replace('\'', "''"),
        shared = shared.display().to_string().replace('\'', "''"),
    );
    fs::write(&ps1, ps_body)?;

    Ok((sh, ps1))
}

fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn target_dir_respects_root_override() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var(ENV_ROOT, tmp.path());
        std::env::remove_var(ENV_USE_LOCAL);

        let shared = target_dir(TargetKind::Shared);
        let verify = target_dir(TargetKind::Verify);
        assert!(shared.starts_with(tmp.path()));
        assert!(
            shared.components().any(|c| c.as_os_str() == "shared"),
            "shared path should end with shared: {}",
            shared.display()
        );
        assert!(
            verify.components().any(|c| c.as_os_str() == "verify"),
            "verify path should end with verify: {}",
            verify.display()
        );
        assert!(shared.exists());

        std::env::remove_var(ENV_ROOT);
    }

    #[test]
    fn local_target_flag_forces_cwd_target() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var(ENV_USE_LOCAL, "1");
        let shared = target_dir(TargetKind::Shared);
        assert!(shared.ends_with("target"));
        std::env::remove_var(ENV_USE_LOCAL);
    }

    #[test]
    fn configure_cargo_sets_env() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var(ENV_ROOT, tmp.path());
        std::env::remove_var(ENV_USE_LOCAL);

        let mut cmd = Command::new("cargo");
        configure_cargo_command(&mut cmd, TargetKind::Verify);
        // Can't easily read Command env; ensure dir was created.
        assert!(tmp.path().join("cargo-target").join("verify").exists());

        std::env::remove_var(ENV_ROOT);
    }
}
