//! Module loader: resolves `use`/`mod` to files so multi-file projects run unchanged.
//!
//! Resolution mirrors the compiler's flat layout: module `a` is `a.wj` or `a/mod.wj`,
//! looked up next to the importing file, then in the project root (and its `src/`).

use super::engine::*;
use super::error::{ErrorKind, RuntimeError};
use crate::parser::Program;
use std::path::{Path, PathBuf};

/// Parse `source`. The returned `Program` owns its arenas (the parser can be dropped).
pub fn parse_source(source: &str, file: &str) -> Result<Program<'static>, String> {
    let mut lex = crate::lexer::Lexer::new(source);
    let tokens = lex.tokenize_with_locations();
    let program = crate::parser::Parser::new_with_source(tokens, file.to_string(), source.to_string()).parse()?;
    // `std::arena` (ADR-0016): same feature gate and escape check as the compiler.
    crate::analyzer::arena_escape::gate(&program)?;
    Ok(program)
}

impl Interpreter {
    /// Load and run an entry file (with its transitive module files).
    pub fn load_entry_file(&mut self, path: &Path) -> Result<(), RuntimeError> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot read {}: {}", path.display(), e)))?;
        let program = parse_source(&src, &path.to_string_lossy())
            .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("parse error: {}", e)))?;
        let prog: &'static Program<'static> = Box::leak(Box::new(program));
        self.modules[0].file = path.to_path_buf();
        self.modules[0].name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        self.load_program(0, prog)
    }

    fn module_search_dirs(&self, from: usize) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        let f = &self.modules[from].file;
        if let Some(d) = f.parent() {
            dirs.push(d.to_path_buf());
        }
        let root = self.modules[0].file.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        dirs.push(root.clone());
        dirs.push(root.join("src"));
        if let Some(p) = root.parent() {
            dirs.push(p.join("src"));
        }
        dirs.dedup();
        dirs
    }

    fn find_module_file(&self, from: usize, name: &str) -> Option<PathBuf> {
        for d in self.module_search_dirs(from) {
            let a = d.join(format!("{}.wj", name));
            if a.is_file() {
                return Some(a);
            }
            let b = d.join(name).join("mod.wj");
            if b.is_file() {
                return Some(b);
            }
        }
        None
    }

    /// Load (or fetch the cached) child module `name` of `from`.
    pub(crate) fn load_child_module(&mut self, from: usize, name: &str) -> Result<Option<usize>, RuntimeError> {
        if let Some(&m) = self.modules[from].children.get(name) {
            return Ok(Some(m));
        }
        if self.missing_modules.contains(&(from, name.to_string())) {
            return Ok(None);
        }
        let Some(file) = self.find_module_file(from, name) else {
            self.missing_modules.insert((from, name.to_string()));
            return Ok(None);
        };
        let canon = file.canonicalize().unwrap_or(file.clone());
        for (i, m) in self.modules.iter().enumerate() {
            if m.file.canonicalize().unwrap_or(m.file.clone()) == canon && i != 0 {
                return Ok(Some(i));
            }
        }
        let src = std::fs::read_to_string(&file)
            .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot read {}: {}", file.display(), e)))?;
        let program = parse_source(&src, &file.to_string_lossy()).map_err(|e| {
            RuntimeError::new(ErrorKind::Semantic, format!("parse error in {}: {}", file.display(), e))
        })?;
        let prog: &'static Program<'static> = Box::leak(Box::new(program));
        let id = self.new_module(name, file, Some(from));
        self.modules[from].children.insert(name.to_string(), id);
        self.register_items(id, &prog.items)?;
        Ok(Some(id))
    }

    /// Resolve one `use` declaration into the module's import table.
    pub(crate) fn register_use(
        &mut self,
        module: usize,
        path: &[String],
        alias: Option<&str>,
    ) -> Result<(), RuntimeError> {
        // Braced imports arrive as one segment "a::b::{X, Y}".
        if path.len() == 1 && path[0].contains("::{") {
            let (prefix, rest) = path[0].split_once("::{").unwrap();
            let names: Vec<&str> = rest.trim_end_matches('}').split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let base: Vec<String> = prefix.split("::").map(|s| s.to_string()).collect();
            for n in names {
                let mut p = base.clone();
                p.push(n.to_string());
                self.register_use(module, &p, None)?;
            }
            return Ok(());
        }
        if path.is_empty() {
            return Ok(());
        }
        let mut segs: Vec<String> = path.iter().map(|s| s.trim_start_matches("./").to_string()).collect();
        if segs[0] == "std" || segs[0] == "core" || segs[0] == "alloc" {
            let local = alias.map(|s| s.to_string()).unwrap_or_else(|| segs.last().unwrap().clone());
            if local != "*" {
                self.modules[module].imports.insert(local, Import::Std(segs.join("::")));
            }
            return Ok(());
        }
        // Resolve the first segment to a starting module.
        let mut cur = module;
        let mut start = 0;
        match segs[0].as_str() {
            "crate" => {
                cur = 0;
                start = 1;
            }
            "self" => start = 1,
            "super" => {
                cur = self.modules[module].parent.unwrap_or(0);
                start = 1;
            }
            _ => {}
        }
        let glob = segs.last().map(|s| s == "*").unwrap_or(false);
        if glob {
            segs.pop();
        }
        let mut i = start;
        while i < segs.len() {
            let seg = segs[i].clone();
            let last = i + 1 == segs.len();
            // Is it a module?
            let child = if cur == 0 && i == start && self.modules[0].children.get(&seg).is_none() {
                self.load_child_module(0, &seg)?
            } else {
                self.load_child_module(cur, &seg)?
            };
            let child = match child {
                Some(c) => Some(c),
                None => match self.modules[cur].imports.get(&seg) {
                    Some(Import::Module(m)) => Some(*m),
                    _ => None,
                },
            };
            if let Some(c) = child {
                if last && !glob {
                    let local = alias.map(|s| s.to_string()).unwrap_or(seg);
                    self.modules[module].imports.insert(local, Import::Module(c));
                }
                cur = c;
                if last && glob {
                    self.import_glob(module, c);
                }
            } else {
                // an item inside `cur`
                if !last {
                    // e.g. `use Enum::Variant` or type path; keep as item import of the type
                }
                if last {
                    let local = alias.map(|s| s.to_string()).unwrap_or(seg.clone());
                    self.modules[module].imports.insert(local, Import::Item(cur, seg));
                }
                if !last {
                    // `use Color::Red`: import the variant name as an item path
                    let full = segs[i..].join("::");
                    let local = alias.map(|s| s.to_string()).unwrap_or(segs.last().unwrap().clone());
                    self.modules[module].imports.insert(local, Import::Item(cur, full));
                }
                break;
            }
            i += 1;
        }
        Ok(())
    }

    fn import_glob(&mut self, into: usize, from: usize) {
        let names: Vec<String> = self.modules[from].funcs.keys().cloned().collect();
        for n in names {
            self.modules[into].imports.entry(n.clone()).or_insert(Import::Item(from, n));
        }
        let consts: Vec<String> = self.modules[from].consts.keys().cloned().collect();
        for n in consts {
            self.modules[into].imports.entry(n.clone()).or_insert(Import::Item(from, n));
        }
        let kids: Vec<(String, usize)> = self.modules[from].children.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (n, m) in kids {
            self.modules[into].imports.entry(n).or_insert(Import::Module(m));
        }
    }
}
