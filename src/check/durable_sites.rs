//! Where the journaled steps of a `durable fn` are (ADR-0021 E4): the interpreter runs the program and journals the calls at
//! these positions. Computed with the same front end as `wj check`, so the interpreter itself never imports `check`.

use std::path::{Path, PathBuf};

/// The positions of the journaled steps of the program rooted at `entry` (empty when it has no `durable fn`).
pub fn discover(entry: &Path) -> Result<Vec<(PathBuf, usize, usize)>, String> {
    use super::effects::{analyze_project, parse_app_inputs};
    let target = match super::effects::manifest_path(entry) {
        Some(m) => m.parent().map(Path::to_path_buf).unwrap_or_else(|| entry.to_path_buf()),
        None => entry.to_path_buf(),
    };
    let files = super::files::collect(&target).map_err(|e| e.to_string())?;
    let mentions = files.iter().any(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains("durable fn")));
    if !mentions {
        return Ok(Vec::new());
    }
    let inputs = parse_app_inputs(&target).map_err(|e| e.to_string())?;
    let a = analyze_project(&target, inputs, false);
    let d = crate::ir::effect_facts::durable::analyze(&a.resolved, &a.facts);
    Ok(d.step_sites.into_iter().collect())
}


