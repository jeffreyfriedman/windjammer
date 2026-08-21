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
}

impl DependencyGraph {
    pub fn build(
        sources: &[(PathBuf, String)],
        parsed_programs: &[Program<'static>],
        src_base: &Path,
    ) -> Self {
        let mut module_to_index: HashMap<Vec<String>, usize> = HashMap::new();
        for (i, (file, _)) in sources.iter().enumerate() {
            if let Some(module_path) =
                crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
            {
                module_to_index.insert(module_path, i);
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
                if let Some(dep_idx) = resolve_import(&file_module, &import_path, &module_to_index)
                {
                    deps.insert(dep_idx);
                }
            }
            // `pub mod child;` (empty body) → sibling `child.wj` must codegen first so
            // re-exports and cross-module callers see refreshed owned/`&str` formals
            // (ecosystem wj-sitegen: domain/mod → domain/render before adapters/fs_site).
            for child in collect_external_submodule_names(&program.items) {
                let mut child_path = file_module.clone();
                child_path.push(child);
                if let Some(&dep_idx) = module_to_index.get(&child_path) {
                    deps.insert(dep_idx);
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
        }
    }

    /// Topological order for codegen: dependencies before importers.
    pub fn sort_indices_for_codegen(&self, indices: &[usize]) -> Vec<usize> {
        let set: HashSet<usize> = indices.iter().copied().collect();
        if set.is_empty() {
            return Vec::new();
        }
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        for &idx in &set {
            let count = self
                .depends_on
                .get(&idx)
                .map(|d| d.iter().filter(|x| set.contains(x)).count())
                .unwrap_or(0);
            in_degree.insert(idx, count);
        }
        let mut zero_deps: Vec<usize> = in_degree
            .iter()
            .filter(|(_, &c)| c == 0)
            .map(|(&i, _)| i)
            .collect();
        zero_deps.sort_unstable();
        let mut queue: VecDeque<usize> = zero_deps.into();
        let mut sorted = Vec::with_capacity(indices.len());
        while let Some(idx) = queue.pop_front() {
            sorted.push(idx);
            if let Some(importers) = self.reverse.get(&idx) {
                for &imp in importers {
                    if set.contains(&imp) {
                        if let Some(deg) = in_degree.get_mut(&imp) {
                            *deg = deg.saturating_sub(1);
                            if *deg == 0 {
                                queue.push_back(imp);
                            }
                        }
                    }
                }
            }
        }
        for &idx in indices {
            if !sorted.contains(&idx) {
                sorted.push(idx);
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
) -> Option<usize> {
    let import_path = normalize_import_path_for_module_resolve(import_path);
    if import_path.is_empty() {
        return None;
    }
    if import_path[0] == "crate" {
        let mut resolved: Vec<String> = import_path[1..].to_vec();
        // `use crate::memory_engine::MemoryEngine` depends on module `memory_engine`,
        // not a fictitious `memory_engine::MemoryEngine` module path.
        while !resolved.is_empty() {
            if let Some(&idx) = module_to_index.get(&resolved) {
                return Some(idx);
            }
            resolved.pop();
        }
        return None;
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
        let mut resolved = base;
        while !resolved.is_empty() {
            if let Some(&idx) = module_to_index.get(&resolved) {
                return Some(idx);
            }
            resolved.pop();
        }
        return None;
    }
    // `use squad::Squad` depends on module `squad`, not a fictitious `squad::Squad` path.
    let mut resolved = import_path;
    while !resolved.is_empty() {
        if let Some(&idx) = module_to_index.get(&resolved) {
            return Some(idx);
        }
        resolved.pop();
    }
    None
}
