//! Dependency metadata roots, filesystem discovery, and type/submodule maps for library builds.

use crate::parser::ast::core::Item;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Map `(parent_module, symbol)` → child module for symbols defined under `parent/child/*.wj`.
/// Fixes `parent::symbol` call sites when Rust places the item in `parent::child`.
pub(crate) fn build_extern_submodule_qualifier_map(
    sources: &[(PathBuf, String)],
    base: &Path,
) -> Result<HashMap<(String, String), String>> {
    build_extern_submodule_qualifier_map_with_programs(sources, base, None)
}

pub(crate) fn build_extern_submodule_qualifier_map_with_programs(
    sources: &[(PathBuf, String)],
    base: &Path,
    parsed_programs: Option<&[crate::parser::Program<'static>]>,
) -> Result<HashMap<(String, String), String>> {
    let mut map: HashMap<(String, String), String> = HashMap::new();
    let mut conflicts: HashSet<(String, String)> = HashSet::new();

    fn merge_extern_submodule_symbols_from_items(
        items: &[Item<'_>],
        module_prefix: &[String],
        map: &mut HashMap<(String, String), String>,
        conflicts: &mut HashSet<(String, String)>,
    ) {
        for item in items {
            match item {
                Item::Function { decl, .. } if decl.is_extern => {
                    insert_extern_submodule_entry(map, conflicts, module_prefix, &decl.name);
                }
                Item::Struct { decl, .. } => {
                    insert_extern_submodule_entry(map, conflicts, module_prefix, &decl.name);
                }
                Item::Enum { decl, .. } => {
                    insert_extern_submodule_entry(map, conflicts, module_prefix, &decl.name);
                }
                Item::Mod {
                    name,
                    items: nested,
                    ..
                } => {
                    let mut next = module_prefix.to_vec();
                    next.push(name.clone());
                    merge_extern_submodule_symbols_from_items(nested, &next, map, conflicts);
                }
                _ => {}
            }
        }
    }

    fn insert_extern_submodule_entry(
        map: &mut HashMap<(String, String), String>,
        conflicts: &mut HashSet<(String, String)>,
        module_prefix: &[String],
        symbol: &str,
    ) {
        if module_prefix.len() < 2 {
            return;
        }
        let parent = module_prefix[module_prefix.len() - 2].clone();
        let child = module_prefix.last().unwrap().clone();
        let key = (parent, symbol.to_string());
        if conflicts.contains(&key) {
            return;
        }
        match map.get(&key) {
            Some(existing) if existing != &child => {
                map.remove(&key);
                conflicts.insert(key);
            }
            Some(_) => {}
            None => {
                map.insert(key, child);
            }
        }
    }

    for (i, (file, source)) in sources.iter().enumerate() {
        let Some(module_path) = crate::analyzer::type_collector::wj_file_to_module_path(base, file)
        else {
            continue;
        };
        if let Some(programs) = parsed_programs {
            merge_extern_submodule_symbols_from_items(
                &programs[i].items,
                &module_path,
                &mut map,
                &mut conflicts,
            );
        } else {
            let (_parser, program) = super::parse_wj_source(file, source)?;
            merge_extern_submodule_symbols_from_items(
                &program.items,
                &module_path,
                &mut map,
                &mut conflicts,
            );
        }
    }

    for k in conflicts {
        map.remove(&k);
    }

    Ok(map)
}

/// Map struct/enum/trait/type-alias names to Rust module paths (from library root) for auto-import resolution.
pub(crate) fn build_type_defining_modules_for_library(
    sources: &[(PathBuf, String)],
    base: &Path,
) -> Result<HashMap<String, Vec<Vec<String>>>> {
    build_type_defining_modules_for_library_with_programs(sources, base, None)
}

pub(crate) fn build_type_defining_modules_for_library_with_programs(
    sources: &[(PathBuf, String)],
    base: &Path,
    parsed_programs: Option<&[crate::parser::Program<'static>]>,
) -> Result<HashMap<String, Vec<Vec<String>>>> {
    let mut map: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for (i, (file, source)) in sources.iter().enumerate() {
        let program = if let Some(programs) = parsed_programs {
            &programs[i]
        } else {
            // Fallback: no AST cache available, re-parse
            let (_parser, p) = super::parse_wj_source(file, source)?;
            let Some(module_path) =
                crate::analyzer::type_collector::wj_file_to_module_path(base, file)
            else {
                continue;
            };
            for name in crate::analyzer::type_collector::collect_local_type_names(&p) {
                map.entry(name).or_default().push(module_path.clone());
            }
            continue;
        };
        let Some(module_path) = crate::analyzer::type_collector::wj_file_to_module_path(base, file)
        else {
            continue;
        };
        for name in crate::analyzer::type_collector::collect_local_type_names(program) {
            map.entry(name).or_default().push(module_path.clone());
        }
    }
    Ok(map)
}

/// Find dependency metadata roots for cross-crate inference.
///
/// Merge order matters: non-`engine` metadata is loaded first, then `engine` last so
/// converged engine `param_ownership` wins over stale copies embedded in a game's own
/// `metadata.json` from a prior build.
pub(crate) fn find_dependency_metadata_roots(
    file_parent: &Path,
    external_paths: &HashMap<String, PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    let engine_path = external_paths.get("engine").cloned();
    for (name, path) in external_paths {
        if name != "engine" {
            roots.push(path.clone());
        }
    }
    if let Some(engine) = engine_path {
        roots.push(engine);
    }

    // When explicit engine metadata is provided, skip walking sibling `src/` trees —
    // they contain per-file `.wj.meta` caches that can overwrite converged signatures.
    if external_paths.contains_key("engine") {
        return roots;
    }

    let canonical =
        std::fs::canonicalize(file_parent).unwrap_or_else(|_| file_parent.to_path_buf());

    // Find the nearest project root so we don't walk past it into unrelated projects.
    let project_root = crate::metadata::find_project_root(&canonical);

    // No manifest (Cargo.toml / wj.toml) found — this is a temp dir or standalone file.
    // Skip the sibling walk entirely to prevent metadata pollution from unrelated projects.
    let Some(root) = project_root else {
        return roots;
    };

    let mut current = canonical.as_path();

    for _ in 0..6 {
        let Some(parent) = current.parent() else {
            break;
        };

        // Never walk above the project root. Workspace sibling crates (e.g. `engine/src`
        // when building `game-core/src/...`) carry stale `metadata.json` / `.wj.meta`
        // entries that overwrite converged local ownership and break call-site auto-borrow.
        if parent == root || !root.starts_with(parent) {
            break;
        }

        // Peer `src/` trees within the same crate only (e.g. cross-module `.wj.meta`).
        if current.starts_with(&root) && current != root {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_dir() {
                        continue;
                    }
                    if canonical.starts_with(&p) {
                        continue;
                    }
                    let src_dir = p.join("src");
                    if src_dir.is_dir() {
                        roots.push(src_dir);
                    }
                    if let Ok(sub_entries) = std::fs::read_dir(&p) {
                        for sub_entry in sub_entries.flatten() {
                            let sub = sub_entry.path();
                            if sub.is_dir() {
                                let sub_src = sub.join("src");
                                if sub_src.is_dir() {
                                    roots.push(sub_src);
                                }
                            }
                        }
                    }
                }
            }
        }

        current = parent;
    }

    roots
}

pub(crate) fn find_wj_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("wj") {
            if path.file_name().and_then(|n| n.to_str()) == Some("mod.wj") {
                if let Some(parent) = path.parent() {
                    find_wj_files_recursive(parent, &mut files)?;
                } else {
                    files.push(path.to_path_buf());
                }
            } else {
                files.push(path.to_path_buf());
            }
        }
    } else if path.is_dir() {
        find_wj_files_recursive(path, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn find_wj_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path);
        } else if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(
                dir_name,
                "build" | "gen" | "target" | ".git" | "node_modules"
            ) {
                continue;
            }
            find_wj_files_recursive(&path, files)?;
        }
    }
    Ok(())
}
