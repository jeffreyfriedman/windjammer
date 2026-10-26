//! Types are keyed by name inside the interpreter. When two modules declare a struct or enum with the same name
//! (`a::P` and `b::P`), the declaration outside the program root is keyed by its module path (`b::P`), and every
//! written type name (`P`, `b::P`, an imported or renamed `P`) resolves to its key through the module scope.
//! A program with no duplicate names pays one empty-set check; its keys are the plain names, as before.

use super::engine::{Import, Interpreter};
use crate::parser::{Item, Program};
use std::collections::{HashMap, HashSet};

/// The name a type prints as: the last segment of its key.
pub(crate) fn display_type_name(key: &str) -> &str {
    key.rsplit("::").next().unwrap_or(key)
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}::{name}")
    }
}

/// Struct and enum names declared in more than one module among `files` (`(module path, items)`), and the key
/// of every declaration of such a name.
pub(crate) fn duplicate_declarations(files: &[(String, &[Item<'_>])]) -> (HashSet<String>, HashSet<String>) {
    fn walk(path: &str, items: &[Item<'_>], seen: &mut HashMap<String, HashSet<String>>) {
        for it in items {
            match it {
                Item::Struct { decl, .. } => {
                    seen.entry(decl.name.clone()).or_default().insert(path.to_string());
                }
                Item::Enum { decl, .. } => {
                    seen.entry(decl.name.clone()).or_default().insert(path.to_string());
                }
                Item::Mod { name, items, .. } => walk(&join(path, name), items, seen),
                _ => {}
            }
        }
    }
    let mut seen: HashMap<String, HashSet<String>> = HashMap::new();
    for (path, items) in files {
        walk(path, items, &mut seen);
    }
    let mut names = HashSet::new();
    let mut decls = HashSet::new();
    for (name, paths) in seen {
        if paths.len() > 1 {
            decls.extend(paths.iter().map(|p| join(p, &name)));
            names.insert(name);
        }
    }
    (names, decls)
}

impl Interpreter {
    /// Record the duplicate declarations of the program about to be registered.
    pub(crate) fn note_duplicate_types(&mut self, files: &[(String, &[Item<'_>])]) {
        let (names, decls) = duplicate_declarations(files);
        self.dup_types.extend(names);
        self.dup_decls.extend(decls);
    }

    pub(crate) fn note_duplicate_types_of(&mut self, program: &Program<'_>) {
        self.note_duplicate_types(&[(String::new(), &program.items)]);
    }

    /// `a::b` for module `m` (empty for the program root).
    pub(crate) fn module_path_string(&self, mut m: usize) -> String {
        let mut parts = Vec::new();
        while let Some(p) = self.modules[m].parent {
            parts.push(self.modules[m].name.clone());
            m = p;
        }
        parts.reverse();
        parts.join("::")
    }

    /// The key under which module `module` declares struct/enum `name`.
    pub(crate) fn declared_type_key(&self, module: usize, name: &str) -> String {
        if self.dup_types.is_empty() || !self.dup_types.contains(name) {
            return name.to_string();
        }
        join(&self.module_path_string(module), name)
    }

    /// The key of the type that `written` (`P`, `b::P`, `crate::b::P`, an alias bound by `use`) denotes in `module`.
    pub(crate) fn type_key(&self, module: usize, written: &str) -> String {
        if self.dup_types.is_empty() {
            return written.to_string();
        }
        let segs: Vec<&str> = written.split("::").collect();
        let Some((last, prefix)) = segs.split_last() else { return written.to_string() };
        let mut m = module;
        for seg in prefix {
            m = match *seg {
                "crate" => self.crate_root_module(m),
                "self" => m,
                "super" => self.modules[m].parent.unwrap_or(0),
                s => match self.modules[m].children.get(s) {
                    Some(&c) => c,
                    None => match self.modules[m].imports.get(s) {
                        Some(Import::Module(c)) | Some(Import::Both(c, _, _)) => *c,
                        _ => return (*last).to_string(),
                    },
                },
            };
        }
        self.type_key_in(m, last, 0)
    }

    fn type_key_in(&self, m: usize, name: &str, depth: usize) -> String {
        let own = join(&self.module_path_string(m), name);
        if self.dup_decls.contains(&own) {
            return own;
        }
        if depth < 8 {
            match self.modules[m].imports.get(name) {
                Some(Import::Item(m2, orig)) | Some(Import::Both(_, m2, orig)) if !(*m2 == m && orig == name) => {
                    return self.type_key_in(*m2, orig, depth + 1);
                }
                _ => {}
            }
            if let Some(fb) = self.modules[m].fallback {
                return self.type_key_in(fb, name, depth + 1);
            }
        }
        name.to_string()
    }

    /// Does the type name written in a pattern or path denote type `key`?
    pub(crate) fn written_type_is(&self, written: &str, key: &str) -> bool {
        if self.dup_types.is_empty() {
            return display_type_name(written) == key;
        }
        self.type_key(self.scope.module, written) == key
    }
}
