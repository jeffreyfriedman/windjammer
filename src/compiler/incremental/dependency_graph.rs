//! Module import dependency graph for incremental reanalysis.

use crate::parser::ast::core::Item;
use crate::parser::Program;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Import edges: file index → indices of files it directly imports.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Reverse edges for transitive dependent lookup (importer → imported)
    reverse: HashMap<usize, HashSet<usize>>,
    /// Forward edges: file index → indices it directly depends on (imports)
    depends_on: HashMap<usize, HashSet<usize>>,
    /// File indices whose stem is `mod.wj` (package directory modules).
    dir_mod_indices: HashSet<usize>,
}

impl DependencyGraph {
    pub fn build(
        sources: &[(PathBuf, String)],
        parsed_programs: &[Program<'static>],
        src_base: &Path,
    ) -> Self {
        let mut module_to_index: HashMap<Vec<String>, usize> = HashMap::new();
        let mut dir_mod_indices: HashSet<usize> = HashSet::new();
        for (i, (file, _)) in sources.iter().enumerate() {
            if let Some(module_path) =
                crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
            {
                module_to_index.insert(module_path, i);
            }
            if file.file_stem().and_then(|s| s.to_str()) == Some("mod") {
                dir_mod_indices.insert(i);
            }
        }

        // Item name → defining module indices (structs/fns/enums/…). Used so
        // `use crate::graph::Port` depends on `graph/types.wj`, not `graph/mod.wj`
        // (re-export parent), avoiding child↔parent codegen cycles.
        let mut item_to_modules: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, program) in parsed_programs.iter().enumerate() {
            for name in collect_defined_item_names(&program.items) {
                item_to_modules.entry(name).or_default().push(i);
            }
        }

        let mut edges: HashMap<usize, HashSet<usize>> = HashMap::new();
        for (i, program) in parsed_programs.iter().enumerate() {
            let file_module =
                crate::analyzer::type_collector::wj_file_to_module_path(src_base, &sources[i].0)
                    .unwrap_or_default();
            let imported = collect_imported_modules(&program.items);
            let mut deps = HashSet::new();
            for import_path in imported {
                if let Some(dep_idx) = resolve_import(
                    &file_module,
                    &import_path,
                    &module_to_index,
                    &dir_mod_indices,
                    &item_to_modules,
                ) {
                    if dep_idx != i {
                        deps.insert(dep_idx);
                    }
                }
            }
            // `pub mod child;` (empty body) → sibling `child.wj` must codegen first so
            // re-exports and cross-module callers see refreshed owned/`&str` formals
            // (ecosystem wj-sitegen: domain/mod → domain/render before adapters/fs_site).
            for child in collect_external_submodule_names(&program.items) {
                let mut child_path = file_module.clone();
                child_path.push(child);
                if let Some(&dep_idx) = module_to_index.get(&child_path) {
                    if dep_idx != i {
                        deps.insert(dep_idx);
                    }
                }
            }
            if !deps.is_empty() {
                edges.insert(i, deps);
            }
        }

        let mut reverse: HashMap<usize, HashSet<usize>> = HashMap::new();
        for (&from, to_set) in &edges {
            for &to in to_set {
                reverse.entry(to).or_default().insert(from);
            }
        }

        Self {
            reverse,
            depends_on: edges,
            dir_mod_indices,
        }
    }

    /// Test/debug: forward dependency edges.
    pub fn depends_on_for_tests(&self) -> &HashMap<usize, HashSet<usize>> {
        &self.depends_on
    }

    /// Topological order for codegen: dependencies before importers.
    ///
    /// Edges *into* package `mod.wj` files are ignored for ordering. Those edges often
    /// come from short re-export imports (`use crate::graph::Port`) and form cycles with
    /// `pub mod child` (`mod → child` + `child → mod`), which collapses a naive Kahn sort
    /// to discovery order (session before engine). Incremental dirty-set analysis still
    /// uses the full [`Self::depends_on`] / [`Self::reverse`] graphs.
    ///
    /// When a true SCC remains after ignoring `mod.wj` edges, break the cycle by emitting
    /// the most-depended-upon pending module first so defining-fn signature refresh reaches
    /// importers (wdb-layers: `graph_bfs_engine` before `graph_analytics_session`).
    pub fn sort_indices_for_codegen(&self, indices: &[usize]) -> Vec<usize> {
        let set: HashSet<usize> = indices.iter().copied().collect();
        if set.is_empty() {
            return Vec::new();
        }
        let counts_dep = |dep: usize| -> bool {
            set.contains(&dep) && !self.dir_mod_indices.contains(&dep)
        };
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        for &idx in &set {
            let count = self
                .depends_on
                .get(&idx)
                .map(|d| d.iter().copied().filter(|&dep| counts_dep(dep)).count())
                .unwrap_or(0);
            in_degree.insert(idx, count);
        }
        let mut pending: HashSet<usize> = set.clone();
        let mut sorted = Vec::with_capacity(indices.len());

        while !pending.is_empty() {
            let mut zeros: Vec<usize> = pending
                .iter()
                .copied()
                .filter(|i| in_degree.get(i).copied().unwrap_or(0) == 0)
                .collect();
            let pick = if !zeros.is_empty() {
                zeros.sort_by_key(|i| (self.dir_mod_indices.contains(i), *i));
                zeros[0]
            } else {
                // SCC: prefer callees with the most pending importers so owned-formal
                // refresh lands before call sites. Then lowest remaining in-degree,
                // then non-`mod.wj`, then stable index.
                let mut best: Option<usize> = None;
                let mut best_key: Option<(std::cmp::Reverse<usize>, usize, bool, usize)> = None;
                for &i in &pending {
                    let rev = self
                        .reverse
                        .get(&i)
                        .map(|imps| imps.iter().filter(|imp| pending.contains(imp)).count())
                        .unwrap_or(0);
                    let deg = in_degree.get(&i).copied().unwrap_or(0);
                    let key = (
                        std::cmp::Reverse(rev),
                        deg,
                        self.dir_mod_indices.contains(&i),
                        i,
                    );
                    if best_key.map(|k| key < k).unwrap_or(true) {
                        best_key = Some(key);
                        best = Some(i);
                    }
                }
                best.expect("pending non-empty")
            };

            pending.remove(&pick);
            sorted.push(pick);
            if let Some(importers) = self.reverse.get(&pick) {
                for &imp in importers {
                    if !pending.contains(&imp) || self.dir_mod_indices.contains(&pick) {
                        continue;
                    }
                    if let Some(deg) = in_degree.get_mut(&imp) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }
        sorted
    }

    /// All file indices that transitively depend on any of `dirty` (includes dirty themselves).
    pub fn transitive_dependents(&self, dirty: &HashSet<usize>) -> HashSet<usize> {
        let mut result = dirty.clone();
        let mut queue: VecDeque<usize> = dirty.iter().copied().collect();
        while let Some(idx) = queue.pop_front() {
            if let Some(importers) = self.reverse.get(&idx) {
                for &importer in importers {
                    if result.insert(importer) {
                        queue.push_back(importer);
                    }
                }
            }
        }
        result
    }
}

/// Names of `mod child;` / `pub mod child;` declarations that refer to a sibling `.wj`
/// file (empty item list). Inline `mod child { ... }` bodies are skipped.
fn collect_external_submodule_names(items: &[Item<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        match item {
            Item::Mod { name, items, .. } if items.is_empty() => {
                names.push(name.clone());
            }
            Item::Mod { items, .. } => {
                names.extend(collect_external_submodule_names(items));
            }
            _ => {}
        }
    }
    names
}

fn collect_defined_item_names(items: &[Item<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        match item {
            Item::Function { decl, .. } => names.push(decl.name.clone()),
            Item::Struct { decl, .. } => names.push(decl.name.clone()),
            Item::Enum { decl, .. } => names.push(decl.name.clone()),
            Item::Trait { decl, .. } => names.push(decl.name.clone()),
            Item::Const { name, .. } | Item::Static { name, .. } | Item::ExternLet { name, .. } => {
                names.push(name.clone());
            }
            Item::Mod { items, .. } => names.extend(collect_defined_item_names(items)),
            // `pub use types::Port` — re-export name is not a definition; defining
            // module is found via the item index from `types.wj`.
            Item::Use { .. } | Item::Impl { .. } | Item::Macro { .. } | Item::BoundAlias { .. } => {}
            _ => {}
        }
    }
    names
}

fn collect_imported_modules(items: &[Item<'_>]) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    for item in items {
        match item {
            Item::Use { path, .. } => {
                paths.push(path.clone());
            }
            Item::Mod { items, .. } => {
                paths.extend(collect_imported_modules(items));
            }
            _ => {}
        }
    }
    paths
}

/// Normalize a `use` path for module-level dependency resolution.
///
/// The parser keeps braced imports as a single segment
/// (`"crate::analytics::{A, B}"`). Dependency edges must target the defining
/// *module* (`["analytics"]`), not the brace list.
fn normalize_import_path_for_module_resolve(import_path: &[String]) -> Vec<String> {
    if import_path.is_empty() {
        return Vec::new();
    }
    if import_path.len() == 1 && import_path[0].contains("::{") {
        let module_part = import_path[0]
            .split("::{")
            .next()
            .unwrap_or(import_path[0].as_str());
        return module_part
            .split("::")
            .filter(|p| !p.is_empty())
            .map(|s| s.to_string())
            .collect();
    }
    let mut out = Vec::with_capacity(import_path.len());
    for seg in import_path {
        if seg.starts_with('{') {
            break;
        }
        if let Some(idx) = seg.find("::{") {
            let head = &seg[..idx];
            if !head.is_empty() {
                out.push(head.to_string());
            }
            break;
        }
        out.push(seg.clone());
    }
    if out.is_empty() {
        import_path.to_vec()
    } else {
        out
    }
}

fn resolve_import(
    current_module: &[String],
    import_path: &[String],
    module_to_index: &HashMap<Vec<String>, usize>,
    dir_mod_indices: &HashSet<usize>,
    item_to_modules: &HashMap<String, Vec<usize>>,
) -> Option<usize> {
    let import_path = normalize_import_path_for_module_resolve(import_path);
    if import_path.is_empty() {
        return None;
    }
    if import_path[0] == "crate" {
        return resolve_crate_or_absolute_path(
            &import_path[1..],
            module_to_index,
            dir_mod_indices,
            item_to_modules,
        );
    }
    if import_path[0] == "super" {
        let mut base = current_module.to_vec();
        // Leading `super::` ascends one module; segments after `import_path[0]` are
        // siblings/cousins under that parent (`datatable` + `super::table` → `table`).
        if !base.is_empty() {
            base.pop();
        }
        for segment in &import_path[1..] {
            if segment == "super" {
                if !base.is_empty() {
                    base.pop();
                }
            } else {
                base.push(segment.clone());
            }
        }
        return resolve_crate_or_absolute_path(
            &base,
            module_to_index,
            dir_mod_indices,
            item_to_modules,
        );
    }
    // `use squad::Squad` depends on module `squad`, not a fictitious `squad::Squad` path.
    resolve_crate_or_absolute_path(
        &import_path,
        module_to_index,
        dir_mod_indices,
        item_to_modules,
    )
}

fn resolve_crate_or_absolute_path(
    path: &[String],
    module_to_index: &HashMap<Vec<String>, usize>,
    dir_mod_indices: &HashSet<usize>,
    item_to_modules: &HashMap<String, Vec<usize>>,
) -> Option<usize> {
    if path.is_empty() {
        return None;
    }
    let mut resolved: Vec<String> = path.to_vec();
    let original_len = resolved.len();
    while !resolved.is_empty() {
        if let Some(&idx) = module_to_index.get(&resolved) {
            let popped_item = if resolved.len() < original_len {
                path.get(resolved.len()).cloned()
            } else {
                None
            };
            // `use crate::graph::Port`: longest module match is often `graph` (mod.wj).
            // Prefer the submodule that *defines* `Port` under that package.
            if dir_mod_indices.contains(&idx) {
                if let Some(ref item) = popped_item {
                    if let Some(def) =
                        prefer_defining_module_under_package(item, &resolved, item_to_modules, module_to_index)
                    {
                        return Some(def);
                    }
                }
                // Bare package import `use crate::graph` → mod.wj is fine.
                if popped_item.is_none() {
                    return Some(idx);
                }
                // Item not found in index: skip this dir-mod match and keep popping
                // so we do not create child→parent cycles from failed re-exports.
                resolved.pop();
                continue;
            }
            return Some(idx);
        }
        resolved.pop();
    }
    // Final fallback: item-only import under crate root.
    if let Some(item) = path.last() {
        if let Some(defs) = item_to_modules.get(item) {
            if defs.len() == 1 {
                return Some(defs[0]);
            }
        }
    }
    None
}

fn prefer_defining_module_under_package(
    item: &str,
    package_path: &[String],
    item_to_modules: &HashMap<String, Vec<usize>>,
    module_to_index: &HashMap<Vec<String>, usize>,
) -> Option<usize> {
    let defs = item_to_modules.get(item)?;
    let mut best: Option<(usize, usize)> = None; // (prefix_len, idx)
    for &idx in defs {
        // Invert module_to_index for this idx (small maps; build is one-shot).
        let Some(mod_path) = module_to_index
            .iter()
            .find(|(_, &i)| i == idx)
            .map(|(p, _)| p.clone())
        else {
            continue;
        };
        if mod_path.len() <= package_path.len() {
            continue;
        }
        if mod_path[..package_path.len()] != *package_path {
            continue;
        }
        let prefix = mod_path.len();
        match best {
            None => best = Some((prefix, idx)),
            Some((best_len, _)) if prefix < best_len => best = Some((prefix, idx)),
            Some(_) => {}
        }
    }
    best.map(|(_, idx)| idx)
}
