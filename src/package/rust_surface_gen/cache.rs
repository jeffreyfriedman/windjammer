//! Generated surfaces in the project cache (`.wj-cache/rust-surfaces/<crate>/surface.wji`, git-ignored):
//! regenerated on demand by `wj build` and `wj rust-surface`, never committed. A hand-written
//! `rust/<crate>.wji` wins over the cached one (see `rust_surface::load`), so nothing is generated for it.

use super::locate::{self, CrateSource};
use super::{generate, header, render, stamp, Generated, Mode};
use std::path::{Path, PathBuf};

pub const CACHE_SUBDIR: &str = "rust-surfaces";

/// `.wj-cache/rust-surfaces/<crate>/` of the project at `manifest_dir`.
pub fn crate_dir(manifest_dir: &Path, crate_name: &str) -> PathBuf {
    manifest_dir.join(".wj-cache").join(CACHE_SUBDIR).join(crate_name)
}

/// The cached surface file of `crate_name`.
pub fn surface_path(manifest_dir: &Path, crate_name: &str) -> PathBuf {
    crate_dir(manifest_dir, crate_name).join("surface.wji")
}

/// The source of `crate_name` for the project at `manifest_dir`: the locked version (lock files of
/// [`locate::lock_files`], `out_dir`'s first), else the newest local version `wj.toml` accepts. With
/// `download`, Cargo is asked when nothing is local.
pub fn source_for(manifest_dir: &Path, crate_name: &str, out_dir: Option<&Path>, download: bool) -> Result<CrateSource, String> {
    let entry = locate::dep_entry(manifest_dir, crate_name)
        .ok_or_else(|| format!("`{crate_name}` is not a [dependencies] entry of {}", manifest_dir.join("wj.toml").display()))?;
    let locks = locate::lock_files(manifest_dir, out_dir);
    let locked = locate::locked_version(&locks, &entry.package, entry.req.as_deref());
    let dir = match locate::find_local(manifest_dir, &entry, locked.as_deref()) {
        Some(d) => d,
        None if download => locate::resolve_with_cargo(&manifest_dir.join(".wj-cache").join(CACHE_SUBDIR).join(".resolve"), &entry)?,
        None => {
            let v = locked.map(|v| format!(" {v}")).unwrap_or_default();
            return Err(format!("the source of `{}`{v} is not in the local Cargo registry", entry.package));
        }
    };
    locate::describe(&dir, crate_name, &entry)
}

/// Generate `crate_name` into the cache (always every public function; `--fn` only filters what is shown).
pub fn regenerate(manifest_dir: &Path, src: &CrateSource) -> Result<(Generated, PathBuf), String> {
    let g = generate(src)?;
    let path = surface_path(manifest_dir, &src.name);
    write_if_changed(&path, &render(&g, Mode::Cache, None))?;
    Ok((g, path))
}

pub fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    if std::fs::read_to_string(path).ok().as_deref() == Some(text) {
        return Ok(());
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `wj build`: make sure every Rust crate dependency without a hand-written surface has a fresh cached one.
/// Fresh means generated for the version that would be used, by this generator and pure list. Returns one
/// line per crate generated or not generatable (nothing when everything was already fresh). Never downloads:
/// a crate whose source is not local is reported with the command that fetches it.
pub fn ensure(manifest_dir: &Path, out_dir: Option<&Path>) -> Vec<String> {
    let crates = crate::package::rust_surface::load(manifest_dir);
    let mut notes = Vec::new();
    for name in &crates.names {
        if crate::package::rust_surface::surface_path(manifest_dir, name).is_file() {
            continue;
        }
        let src = match source_for(manifest_dir, name, out_dir, false) {
            Ok(s) => s,
            Err(why) => {
                if !surface_path(manifest_dir, name).is_file() {
                    notes.push(format!("no signature surface for Rust crate `{name}`: {why}; `wj rust-surface {name}` fetches it and generates one"));
                }
                continue;
            }
        };
        let cached = std::fs::read_to_string(surface_path(manifest_dir, name)).ok().and_then(|t| header(&t));
        if cached.is_some_and(|h| h.version == src.version && h.stamp == stamp()) {
            continue;
        }
        match regenerate(manifest_dir, &src) {
            Ok((g, path)) => notes.push(format!(
                "generated the signature surface of Rust crate `{name}` {} ({} functions, {} skipped) into {}; `wj rust-surface {name} --write` pins an editable copy",
                g.version,
                g.fns.len(),
                g.skipped.len(),
                path.strip_prefix(manifest_dir).unwrap_or(&path).display()
            )),
            Err(why) => notes.push(format!("cannot generate the signature surface of Rust crate `{name}`: {why}")),
        }
    }
    notes
}
