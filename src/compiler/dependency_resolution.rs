//! Dependency metadata roots, filesystem discovery, and type/submodule maps for library builds.

use crate::parser::ast::core::Item;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Map `(parent_module, symbol)` → child module for symbols defined under `parent/child/*.wj`.
/// Fixes `parent::symbol` call sites when Rust places the item in `parent::child`.
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

/// Resolve `gen/` (or crate-root) directories that contain `metadata.json` for each
/// `wj.toml` path dependency under `build_path`'s project.
///
/// Keys use Rust crate naming (`substrate-crate` → `substrate_crate`) so they match
/// `--metadata` / `external_paths` conventions. Explicit CLI `--metadata` entries
/// should override these when merged.
/// `Err` names each path dependency with Windjammer sources whose signatures could not be derived: its call sites
/// would otherwise be guessed, so the build stops.
pub(crate) fn discover_wj_toml_path_dependency_metadata(
    build_path: &Path,
) -> Result<HashMap<String, PathBuf>, String> {
    let mut out = HashMap::new();
    let mut errors = Vec::new();
    let search_from = if build_path.is_file() {
        build_path.parent().unwrap_or(build_path)
    } else {
        build_path
    };
    let Some(project_root) = crate::metadata::find_project_root(search_from) else {
        return Ok(out);
    };
    let wj_toml = project_root.join("wj.toml");
    let Ok(config) = crate::config::WjConfig::load_from_file(&wj_toml) else {
        return Ok(out);
    };

    for (name, spec) in config
        .dependencies
        .iter()
        .chain(config.dev_dependencies.iter())
    {
        let path_str = match spec {
            crate::config::DependencySpec::Detailed { path: Some(p), .. } => p.as_str(),
            _ => continue,
        };
        let dep_root = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            project_root.join(path_str)
        };
        let key = name.replace('-', "_");
        // Prefer library gen/ output (post-codegen ownership + emitted_rust_ref_params).
        let candidates = [dep_root.join("gen"), dep_root.clone(), dep_root.join("src")];
        let mut found = None;
        for candidate in &candidates {
            let meta_file = if candidate.is_file()
                && candidate.file_name().is_some_and(|n| n == "metadata.json")
            {
                candidate.clone()
            } else {
                candidate.join("metadata.json")
            };
            if meta_file.is_file() {
                let root = meta_file
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| candidate.clone());
                found = Some(root);
                break;
            }
        }
        // No metadata.json: a dependency with Windjammer sources is analyzed from them (never guessed).
        if found.is_none() {
            // `path = "../pkg/build"` names a package's generated output: its declarations are the package's sources.
            let source_root = super::path_dep_metadata::source_package_of(&dep_root);
            match super::path_dep_metadata::derive(&key, source_root.as_deref().unwrap_or(&dep_root)) {
                Some(Ok(root)) => found = Some(root),
                Some(Err(e)) => errors.push(format!("{e}; its call sites would have no signatures")),
                None => {}
            }
        }
        // Ecosystem `path = "…/build"` often has generated `lib.rs` and no metadata.json.
        if found.is_none() {
            for candidate in &candidates {
                if candidate.join("lib.rs").is_file() {
                    found = Some(candidate.clone());
                    break;
                }
            }
        }
        if let Some(root) = found {
            out.insert(key, root);
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors.join("\n"))
    }
}

/// Discover `metadata.json` roots for crates imported via `use crate_name::...` in `.wj`
/// sources — covers deps present in generated Cargo.toml but missing from `wj.toml`
/// (e.g. `substrate-crate` → `wal_crate` via `use wal_crate::WalSegment`).
pub(crate) fn discover_wj_import_dependency_metadata(
    build_path: &Path,
) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let src_root = if build_path.is_file() {
        build_path.parent().unwrap_or(build_path).to_path_buf()
    } else {
        build_path.to_path_buf()
    };
    let Ok(wj_files) = find_wj_files(&src_root) else {
        return out;
    };
    let project_root =
        crate::metadata::find_project_root(&src_root).unwrap_or_else(|| src_root.clone());
    let output_hint = project_root.join("gen");

    let builtin = [
        "std",
        "core",
        "alloc",
        "crate",
        "self",
        "super",
        "windjammer_runtime",
        "serde",
    ];

    for file in &wj_files {
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("use ") else {
                continue;
            };
            let Some(crate_name) = rest.split("::").next() else {
                continue;
            };
            let crate_name = crate_name.trim();
            if crate_name.is_empty()
                || builtin.contains(&crate_name)
                || !crate_name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase())
            {
                continue;
            }
            let key = crate_name.replace('-', "_");
            if out.contains_key(&key) {
                continue;
            }
            let Some(crate_dir) = crate::cargo_toml::dependency_management::resolve_crate_dir(
                crate_name,
                &src_root,
                &output_hint,
            ) else {
                continue;
            };
            for candidate in [crate_dir.join("gen"), crate_dir.clone()] {
                let meta = candidate.join("metadata.json");
                if meta.is_file() {
                    out.insert(key.clone(), candidate);
                    break;
                }
            }
        }
    }
    out
}

/// [`find_dependency_metadata_roots`] with the crates named explicitly (`--metadata`; the library a
/// test crate imports) last: later roots overwrite bare keys, so a transitive path dependency's
/// homonym cannot take the slot of the crate the program imports from.
pub(crate) fn metadata_roots_explicit_last(
    file_parent: &Path,
    external_paths: &HashMap<String, PathBuf>,
    explicit: &[(&str, &Path)],
) -> Vec<PathBuf> {
    let mut roots = find_dependency_metadata_roots(file_parent, external_paths);
    roots.sort_by_key(|root| explicit.iter().any(|(_, p)| *p == root.as_path()));
    roots
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
    // Name order, not hash order: later roots overwrite bare keys, so the order decides homonyms.
    let mut named: Vec<_> = external_paths.iter().filter(|(name, _)| *name != "engine").collect();
    named.sort();
    roots.extend(named.into_iter().map(|(_, path)| path.clone()));
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
            crate::metadata::scan_stats::dir_listed();
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
                    crate::metadata::scan_stats::dir_listed();
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

/// A build of one file includes the sibling modules it imports (`use name::...` with `name.wj` or `name/mod.wj`
/// beside it), transitively: a program with imports has no other compiled form.
pub(crate) fn with_sibling_imports(path: &Path, mut files: Vec<PathBuf>) -> Vec<PathBuf> {
    if !path.is_file() || files.len() != 1 || path.file_name().and_then(|n| n.to_str()) == Some("mod.wj") {
        return files;
    }
    let Some(dir) = path.parent().map(|d| if d.as_os_str().is_empty() { Path::new(".") } else { d }) else { return files };
    let mut queue = vec![path.to_path_buf()];
    while let Some(file) = queue.pop() {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        for line in text.lines() {
            let l = line.trim_start();
            let Some(rest) = l.strip_prefix("use ").or_else(|| l.strip_prefix("pub use ")) else { continue };
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.is_empty() {
                continue;
            }
            let found = [dir.join(format!("{name}.wj")), dir.join(&name).join("mod.wj")].into_iter().find(|p| p.is_file());
            if let Some(f) = found {
                if !files.contains(&f) {
                    files.push(f.clone());
                    queue.push(f);
                }
            }
        }
    }
    files.sort();
    files
}

/// After a build of some of a directory's files: remove the `mod`/`pub use` lines for the root-level siblings this
/// build left out (a file build declares the modules it imported, not every `.wj` beside it).
pub(crate) fn drop_unbuilt_sibling_modules(output: &Path, base_path: &Path, built: &[PathBuf]) {
    let stem = |p: &Path| p.file_stem().and_then(|s| s.to_str()).map(str::to_string);
    let built: HashSet<String> = built.iter().filter_map(|p| stem(p)).collect();
    let Ok(rd) = std::fs::read_dir(base_path) else { return };
    let left_out: HashSet<String> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "wj"))
        .filter_map(|p| stem(&p))
        .filter(|n| n != "mod" && !built.contains(n))
        .collect();
    if left_out.is_empty() {
        return;
    }
    for f in ["lib.rs", "mod.rs"] {
        let file = output.join(f);
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        let kept: Vec<&str> = text
            .lines()
            .filter(|l| {
                let t = l.trim();
                let name = t
                    .strip_prefix("pub mod ")
                    .or_else(|| t.strip_prefix("mod "))
                    .and_then(|r| r.strip_suffix(';'))
                    .or_else(|| t.strip_prefix("pub use ").and_then(|r| r.strip_suffix("::*;")));
                name.is_none_or(|n| !left_out.contains(n))
            })
            .collect();
        if kept.len() != text.lines().count() {
            let _ = std::fs::write(&file, kept.join("\n") + "\n");
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discover_path_dep_metadata_prefers_gen_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("consumer");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("wj.toml"),
            r#"
[package]
name = "consumer"
version = "0.1.0"

[dependencies]
dep-crate = { path = "../dep_crate" }
"#,
        )
        .unwrap();
        let dep = tmp.path().join("dep_crate");
        fs::create_dir_all(dep.join("gen")).unwrap();
        fs::write(
            dep.join("gen/metadata.json"),
            r#"{"structs":{},"functions":{},"copy_structs":[],"version":"0"}"#,
        )
        .unwrap();
        fs::write(
            dep.join("metadata.json"),
            r#"{"structs":{},"functions":{},"copy_structs":[],"version":"0"}"#,
        )
        .unwrap();

        let found = discover_wj_toml_path_dependency_metadata(&root.join("src")).unwrap();
        assert_eq!(
            found.get("dep_crate").map(|p| p.file_name().unwrap()),
            Some(std::ffi::OsStr::new("gen")),
            "expected gen/ metadata root, got {found:?}"
        );
    }
}

fn find_wj_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    // One walker shared with the interpreter and resolver (module_system::sources).
    files.extend(crate::module_system::sources::wj_files_under(dir)?);
    Ok(())
}
