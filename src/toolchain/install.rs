//! `wj install-dev`: atomically install a `wj` binary (default: the one running) to a managed location
//! (default `$CARGO_HOME/bin/wj`, else `~/.cargo/bin/wj`), keeping a timestamped backup, refusing to
//! downgrade without `--force`, and verifying that the installed binary runs `--version`.

use super::doctor::probe;
use super::identity::Identity;
use super::timefmt::{compact_datetime, now_ts};
use std::path::{Path, PathBuf};

pub const KEEP_BACKUPS: usize = 5;

pub fn default_dest() -> Result<PathBuf, String> {
    if let Some(h) = super::env::get("CARGO_HOME").filter(|h| !h.is_empty()) {
        return Ok(PathBuf::from(h).join("bin").join("wj"));
    }
    let home = super::env::get("HOME")
        .or_else(|| super::env::get("USERPROFILE"))
        .ok_or("cannot determine the home directory")?;
    Ok(PathBuf::from(home).join(".cargo").join("bin").join("wj"))
}

#[derive(Debug)]
pub struct Installed {
    pub dest: PathBuf,
    pub backup: Option<PathBuf>,
    pub identity: Identity,
    pub already_current: bool,
}

/// Would installing `new` over `old` lose recency? Unknown commit dates count as older than known ones.
pub fn is_downgrade(new: &Identity, old: &Identity) -> bool {
    match (new.commit_ts, old.commit_ts) {
        (Some(n), Some(o)) => n < o,
        (None, Some(_)) => true,
        _ => false,
    }
}

fn same_build(a: &Identity, b: &Identity) -> bool {
    a.commit.is_some() && a.commit == b.commit && a.dirty == b.dirty && a.build_ts == b.build_ts
}

pub fn install(src: &Path, dest: &Path, force: bool, dry_run: bool) -> Result<Installed, String> {
    let new = probe(src).map_err(|e| format!("the binary to install ({}) {e}", src.display()))?;
    let old = dest.is_file().then(|| probe(dest));
    if let Some(Ok(old)) = &old {
        if same_build(&new, old) {
            return Ok(Installed {
                dest: dest.to_path_buf(),
                backup: None,
                identity: new,
                already_current: true,
            });
        }
        if is_downgrade(&new, old) && !force {
            return Err(format!(
                "refusing to downgrade {} (commit {}) to an older build (commit {}); pass --force to do it anyway",
                dest.display(),
                old.short(),
                new.short()
            ));
        }
    }
    let backup = dest.is_file().then(|| {
        let name = dest
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "wj".into());
        dest.with_file_name(format!("{name}.{}.bak", compact_datetime(now_ts())))
    });
    if dry_run {
        return Ok(Installed {
            dest: dest.to_path_buf(),
            backup,
            identity: new,
            already_current: false,
        });
    }
    let dir = dest.parent().ok_or("destination has no parent directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let tmp = dir.join(format!(".wj-install-{}.tmp", std::process::id()));
    let cleanup = |e: String| {
        let _ = std::fs::remove_file(&tmp);
        e
    };
    std::fs::copy(src, &tmp)
        .map_err(|e| cleanup(format!("copy {} -> {}: {e}", src.display(), tmp.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| cleanup(e.to_string()))?;
    }
    let staged =
        probe(&tmp).map_err(|e| cleanup(format!("the staged binary {e}; nothing was changed")))?;
    if staged.commit != new.commit {
        return Err(cleanup(
            "the staged binary reports a different commit than the source; nothing was changed"
                .into(),
        ));
    }
    if let Some(b) = &backup {
        std::fs::copy(dest, b).map_err(|e| cleanup(format!("backup {}: {e}", b.display())))?;
    }
    std::fs::rename(&tmp, dest).map_err(|e| cleanup(format!("rename into place: {e}")))?;
    let installed = probe(dest).map_err(|e| {
        format!(
            "installed {} but it {e}; restore with: cp {} {}",
            dest.display(),
            backup
                .as_ref()
                .map(|b| b.display().to_string())
                .unwrap_or_default(),
            dest.display()
        )
    })?;
    prune_backups(dest);
    Ok(Installed {
        dest: dest.to_path_buf(),
        backup,
        identity: installed,
        already_current: false,
    })
}

fn prune_backups(dest: &Path) {
    let (Some(dir), Some(name)) = (
        dest.parent(),
        dest.file_name().map(|n| n.to_string_lossy().to_string()),
    ) else {
        return;
    };
    let prefix = format!("{name}.");
    let mut baks: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with(&prefix) && n.ends_with(".bak")
            })
        })
        .collect();
    baks.sort();
    let excess = baks.len().saturating_sub(KEEP_BACKUPS);
    for p in baks.into_iter().take(excess) {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(ts: Option<i64>) -> Identity {
        Identity {
            version: "0.50.0".into(),
            commit: Some("abc".into()),
            commit_ts: ts,
            dirty: false,
            profile: None,
            build_ts: None,
        }
    }

    #[test]
    fn downgrade_rules() {
        assert!(is_downgrade(&id(Some(1)), &id(Some(2))));
        assert!(!is_downgrade(&id(Some(2)), &id(Some(1))));
        assert!(!is_downgrade(&id(Some(2)), &id(Some(2))));
        assert!(is_downgrade(&id(None), &id(Some(2))));
        assert!(!is_downgrade(&id(Some(2)), &id(None)));
    }
}
