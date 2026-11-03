// wj rust-surface - generate a Rust crate dependency's signature surface (ADR-0026) from the crate source.
//
// The full surface always goes to the project cache (.wj-cache/rust-surfaces/<crate>/surface.wji, never
// committed). Without --write the surface is printed; with --write an editable copy is written to
// rust/<crate>.wji, which is source and wins over the cached one. --fn restricts what is printed or written.

use crate::package::rust_surface_gen::{cache, render, Mode};
use anyhow::{bail, Result};
use std::path::Path;

pub fn execute(path: &Path, crate_name: &str, only: &[String], write: bool) -> Result<()> {
    let start = if path.is_dir() { path.to_path_buf() } else { path.parent().unwrap_or(Path::new(".")).to_path_buf() };
    let Some(root) = crate::module_system::project::find_manifest_dir(&start) else {
        bail!("no wj.toml found at or above {}", start.display());
    };
    let key = crate_name.replace('-', "_");
    let src = cache::source_for(&root, &key, None, true).map_err(anyhow::Error::msg)?;
    let (g, cache_path) = cache::regenerate(&root, &src).map_err(anyhow::Error::msg)?;
    for name in only {
        if g.fns.iter().any(|f| f.name == *name) {
            continue;
        }
        match g.skipped.iter().find(|(n, _)| n == name) {
            Some((_, why)) => bail!("`{key}::{name}` cannot be generated ({why}); declare it by hand in rust/{key}.wji"),
            None => bail!("`{key}` {} has no public free function `{name}`", g.version),
        }
    }
    let filter = (!only.is_empty()).then_some(only);
    if write {
        let target = crate::package::rust_surface::surface_path(&root, &key);
        if target.is_file() && crate::package::rust_surface_gen::header(&std::fs::read_to_string(&target)?).is_none() {
            bail!("{} is hand-written; move it away first (wj rust-surface does not overwrite hand-written surfaces)", target.display());
        }
        cache::write_if_changed(&target, &render(&g, Mode::Pinned, filter)).map_err(anyhow::Error::msg)?;
        eprintln!("wrote {} (source: edit and commit it; it takes precedence over the generated cache copy)", target.display());
    } else {
        print!("{}", render(&g, Mode::Cache, filter));
    }
    eprintln!(
        "{key} {}: {} function(s) generated, {} skipped; cache copy {}",
        g.version,
        g.fns.len(),
        g.skipped.len(),
        cache_path.strip_prefix(&root).unwrap_or(&cache_path).display()
    );
    Ok(())
}
