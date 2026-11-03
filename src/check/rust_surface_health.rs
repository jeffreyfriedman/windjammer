//! `W0015`: a generated Rust crate surface (cached or a `--write` copy) recorded a crate version other than the
//! one `Cargo.lock` locks, so the surface may describe functions that changed (ADR-0026). Hand-written surfaces
//! have no header and are not judged. Only local files are read: the surface and the project's lock files.

use crate::diagnostics::{Diagnostic, SourceIndex};
use crate::package::rust_surface::RustCrates;
use crate::package::rust_surface_gen::locate;
use std::path::Path;

pub fn run(manifest_dir: &Path, crates: &RustCrates, out: &mut Vec<Diagnostic>) {
    let locks = locate::lock_files(manifest_dir, None);
    if locks.is_empty() {
        return;
    }
    for s in &crates.surfaces {
        let Some(h) = &s.generated else { continue };
        let Some(entry) = locate::dep_entry(manifest_dir, &s.crate_name) else { continue };
        let Some(locked) = locate::locked_version(&locks, &entry.package, entry.req.as_deref()) else { continue };
        if locked == h.version {
            continue;
        }
        let older = match (semver::Version::parse(&h.version), semver::Version::parse(&locked)) {
            (Ok(a), Ok(b)) => a < b,
            _ => false,
        };
        let krate = &s.crate_name;
        let ix = SourceIndex::new(s.path.display().to_string(), &s.text);
        let line_len = s.text.lines().nth(h.line - 1).map_or(1, str::len);
        let d = Diagnostic::warning(
            "W0015",
            format!(
                "the signature surface of Rust crate `{krate}` was generated from {} {}, {} the locked {locked}",
                krate,
                h.version,
                if older { "older than" } else { "not" }
            ),
        )
        .at(ix.span_at(h.line, 1, line_len).with_label("generated from this version"));
        out.push(if h.cache {
            d.with_help(format!("`wj build` or `wj rust-surface {krate}` regenerates it"))
        } else {
            d.with_help(format!("regenerate the pinned copy with `wj rust-surface {krate} --write`, then re-apply your edits"))
        });
    }
}
