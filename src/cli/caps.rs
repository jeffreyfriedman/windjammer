// wj caps - what the project's dependencies are known to do (ADR-0021 per-package grants, WJ-SEC-03 lock file).
//
//   wj caps show [PATH]      inferred effects per dependency, and whether `.wj-capabilities.lock` covers them
//   wj caps approve [PATH]   record the inferred effects of every dependency in `.wj-capabilities.lock`

use crate::check::effects::{analyze_project, parse_app_inputs};
use crate::ir::effect_facts::lock::{package_effects, CapLock, PackageLock};
use anyhow::Result;
use std::path::Path;

fn current(path: &Path) -> Result<(std::path::PathBuf, std::collections::BTreeMap<String, std::collections::BTreeSet<String>>, Option<CapLock>, std::collections::BTreeMap<String, Option<String>>)> {
    let inputs = parse_app_inputs(path)?;
    let a = analyze_project(path, inputs, true);
    let effects = package_effects(&a.resolved, &a.facts, &a.dep_names);
    Ok((a.manifest_dir.clone(), effects, a.lock.clone(), a.dep_revs.clone()))
}

pub fn show(path: &Path, json: bool) -> Result<()> {
    let (_, effects, lock, _) = current(path)?;
    let mut rows = Vec::new();
    for (pkg, refs) in &effects {
        let status = match &lock {
            None => "no lock file".to_string(),
            Some(l) => {
                let esc = l.escalations(&[(pkg.clone(), refs.clone())].into_iter().collect());
                if esc.is_empty() { "locked".to_string() } else { format!("{} not in the lock", esc.len()) }
            }
        };
        rows.push((pkg.clone(), refs.iter().cloned().collect::<Vec<_>>(), status));
    }
    if json {
        let v: Vec<_> = rows.iter().map(|(p, e, s)| serde_json::json!({"package": p, "effects": e, "status": s})).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({"packages": v}))?);
    } else if rows.is_empty() {
        println!("no Windjammer dependencies");
    } else {
        for (p, e, s) in rows {
            println!("{p}: {}  [{s}]", if e.is_empty() { "pure".to_string() } else { e.join(", ") });
        }
    }
    Ok(())
}

pub fn approve(path: &Path) -> Result<()> {
    let (dir, effects, _, revs) = current(path)?;
    let mut lock = CapLock::new();
    for (pkg, refs) in effects {
        let rev = revs.get(&pkg).cloned().flatten();
        lock.packages.insert(pkg, PackageLock { rev, effects: refs });
    }
    lock.save(&dir).map_err(|e| anyhow::anyhow!(e))?;
    println!("wrote {} ({} package(s))", dir.join(crate::ir::effect_facts::lock::LOCK_FILE).display(), lock.packages.len());
    Ok(())
}

pub fn execute(action: &str, path: &Path, json: bool) -> Result<()> {
    match action {
        "show" => show(path, json),
        "approve" => approve(path),
        other => anyhow::bail!("unknown action `{other}` (show, approve)"),
    }
}
