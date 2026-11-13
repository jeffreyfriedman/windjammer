//! Where the journaled steps of a `durable fn` are (ADR-0021 E4): the interpreter runs the program and journals the calls at
//! these positions. Computed with the same front end as `wj check`, so the interpreter itself never imports `check`.

use std::path::{Path, PathBuf};

/// The positions of the journaled steps of the program rooted at `entry` (empty when it has no `durable fn`).
pub fn discover(entry: &Path) -> Result<Vec<(PathBuf, usize, usize)>, String> {
    discover_split(entry).map(|(all, _)| all)
}

/// The step positions, and among them the ones whose call only reads (a compiled build journals those lazily).
#[allow(clippy::type_complexity)]
pub fn discover_split(entry: &Path) -> Result<(Vec<(PathBuf, usize, usize)>, Vec<(PathBuf, usize, usize)>), String> {
    use super::effects::{analyze_project, parse_app_inputs};
    // A `durable fn` the program can call is in its own sources: the entry and the modules it loads. Look there first:
    // the manifest found above the entry may be a stray one far up the tree (a `wj.toml` in a shared temp directory
    // made this scan read every `.wj` file below it, 2.5G instructions for a 200-line program, ADR-0038 agent loop).
    let own = if entry.is_file() && !entry.file_name().is_some_and(|n| n == "main.wj" || n == "lib.wj") { Ok(super::files::reachable(entry)) } else { super::files::collect(entry) };
    let local_mentions = own.map_err(|e| e.to_string())?.iter().any(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains("durable fn")));
    if !local_mentions {
        return Ok((Vec::new(), Vec::new()));
    }
    let target = match super::effects::manifest_path(entry) {
        // a relative `wj.toml` has the empty path as its parent: that is the working directory, not a missing path
        Some(m) => match m.parent() {
            Some(p) if p.as_os_str().is_empty() => PathBuf::from("."),
            Some(p) => p.to_path_buf(),
            None => entry.to_path_buf(),
        },
        None => entry.to_path_buf(),
    };
    let files = super::files::collect(&target).map_err(|e| e.to_string())?;
    let mentions = files.iter().any(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains("durable fn")));
    if !mentions {
        return Ok((Vec::new(), Vec::new()));
    }
    let inputs = parse_app_inputs(&target).map_err(|e| e.to_string())?;
    let a = analyze_project(&target, inputs, false);
    let d = crate::ir::effect_facts::durable::analyze(&a.resolved, &a.facts);
    Ok((d.step_sites.into_iter().collect(), d.read_sites.into_iter().collect()))
}



#[cfg(test)]
mod local_first_tests {
    use super::*;

    /// A stray `wj.toml` far above the entry, with a `durable fn` somewhere else below it, does not make a program
    /// without one scan or analyze that tree; a `durable fn` in a module the entry loads is still found.
    #[test]
    fn a_durable_fn_outside_the_entrys_directory_is_not_the_programs() {
        let root = std::env::temp_dir().join(format!("wj-durable-local-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();
        std::fs::write(root.join("wj.toml"), "[package]\nname = \"stray\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(root.join("other/elsewhere.wj"), "durable fn step() {\n}\n").unwrap();
        let entry = root.join("app/main.wj");
        std::fs::write(&entry, "fn main() {\n    println(\"hi\")\n}\n").unwrap();
        assert_eq!(discover_split(&entry).unwrap(), (Vec::new(), Vec::new()));
        // a module the entry loads is the program's own source, so its `durable fn` is still looked at
        std::fs::write(root.join("app/jobs.wj"), "durable fn job() {\n}\n").unwrap();
        std::fs::write(&entry, "mod jobs\n\nfn main() {\n    println(\"hi\")\n}\n").unwrap();
        assert!(crate::check::files::collect(&entry).unwrap().iter().any(|f| std::fs::read_to_string(f).unwrap().contains("durable fn")));
        let _ = std::fs::remove_dir_all(&root);
    }
}
