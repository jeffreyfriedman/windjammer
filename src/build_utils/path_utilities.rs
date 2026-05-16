//! Path helpers for build output layout (`src/` subtree vs crate root).

use std::path::Path;

/// True when the output directory is a Rust module subtree under `src/...` (not the crate root).
/// In that case we only emit `mod.rs` for Cargo, not `lib.rs` (which would be invalid as a nested crate root).
pub(crate) fn is_submodule_output_dir(output_dir: &Path) -> bool {
    let comps: Vec<_> = output_dir.components().collect();
    for i in 0..comps.len() {
        if comps[i].as_os_str() == "src" && i + 1 < comps.len() {
            return true;
        }
    }
    false
}

pub(crate) fn source_dir_for_output(
    output_subdir: &Path,
    layout: Option<(&Path, &Path)>,
) -> Option<std::path::PathBuf> {
    let (out_root, src_root) = layout?;
    let rel = output_subdir.strip_prefix(out_root).ok()?;
    Some(src_root.join(rel))
}
