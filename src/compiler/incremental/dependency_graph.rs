//! Module import dependency graph for incremental reanalysis.

use crate::parser::ast::core::Item;
use crate::parser::Program;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Import edges: file index → indices of files it directly imports.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// file_index → set of imported file indices
    edges: HashMap<usize, HashSet<usize>>,
    /// Reverse edges for transitive dependent lookup
    reverse: HashMap<usize, HashSet<usize>>,
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
            let file_module = crate::analyzer::type_collector::wj_file_to_module_path(
                src_base,
                &sources[i].0,
            )
            .unwrap_or_default();
            let imported = collect_imported_modules(&program.items);
            let mut deps = HashSet::new();
            for import_path in imported {
                if let Some(dep_idx) =
                    resolve_import(&file_module, &import_path, &module_to_index)
                {
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

        Self { edges, reverse }
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

fn resolve_import(
    current_module: &[String],
    import_path: &[String],
    module_to_index: &HashMap<Vec<String>, usize>,
) -> Option<usize> {
    if import_path.is_empty() {
        return None;
    }
    if import_path[0] == "crate" {
        let resolved: Vec<String> = import_path[1..].to_vec();
        return module_to_index.get(&resolved).copied();
    }
    if import_path[0] == "super" {
        let mut base = current_module.to_vec();
        for segment in &import_path[1..] {
            if segment == "super" {
                base.pop();
            } else {
                base.push(segment.clone());
            }
        }
        return module_to_index.get(&base).copied();
    }
    module_to_index.get(import_path).copied()
}
