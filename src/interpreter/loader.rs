//! Module loader: resolves `use`/`mod` to files so multi-file projects run unchanged.
//!
//! Resolution mirrors the compiler's flat layout: module `a` is `a.wj` or `a/mod.wj`,
//! looked up next to the importing file, then in the project root (and its `src/`).

use super::engine::*;
use super::error::{ErrorKind, RuntimeError};
use crate::parser::Program;
use std::path::{Path, PathBuf};

/// Parse `source`. The AST borrows the parser's arenas, so the parser is intentionally leaked:
/// every `&'static` node handed to the interpreter stays valid for the life of the process.
pub fn parse_source(source: &str, file: &str) -> Result<Program<'static>, String> {
    let mut lex = crate::lexer::Lexer::new(source);
    let tokens = lex.tokenize_with_locations();
    let parse: &'static mut crate::parser::Parser = Box::leak(Box::new(
        crate::parser::Parser::new_with_source(tokens, file.to_string(), source.to_string()),
    ));
    parse.parse()
}

/// The files of the program whose entry is `entry` (entry first), with their module paths.
pub fn program_files(entry: &Path) -> Result<Vec<super::project::LoadedFile>, RuntimeError> {
    use crate::module_system::project;
    let project_root = project::find_manifest_dir(entry);
    let is_crate_root = entry.file_name().is_some_and(|n| n == "main.wj" || n == "lib.wj")
        || project_root.as_deref().is_some_and(|r| project::bin_targets(r).iter().any(|b| same_file(&b.entry, entry)));
    if !is_crate_root {
        return super::project::script_sources(entry);
    }
    let root = project::source_root_for(entry, project_root.as_deref());
    let files = crate::module_system::sources::program_sources(&root, entry)
        .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot read {}: {}", root.display(), e)))?;
    super::project::parse_files(&files)
}

/// The program a test file runs in, and the module path of the file inside it.
///
/// A test file of a project (`wj.toml` above it, crate root `lib.wj`/`main.wj` in its source root) runs
/// against the whole project, exactly as `wj run` and the compiled `wj test` see it, so `use crate::a::P`
/// reaches sibling modules. The source root is `<project>/src` when it exists, else the project directory
/// (the same layout `module_system::project` uses). A file outside any project is a script.
pub fn test_program_files(file: &Path) -> Result<(Vec<super::project::LoadedFile>, Vec<String>, PathBuf), RuntimeError> {
    use crate::module_system::{project, sources::SourceFile};
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let crate_entry = project::find_manifest_dir(&file).and_then(|proj| {
        let root = if proj.join("src").is_dir() { proj.join("src") } else { proj };
        ["lib.wj", "main.wj", "mod.wj"].iter().map(|n| root.join(n)).find(|e| e.is_file()).map(|e| (root, e))
    });
    let Some((root, entry)) = crate_entry.filter(|(_, e)| !same_file(e, &file)) else {
        return Ok((program_files(&file)?, Vec::new(), file));
    };
    let mut sources = crate::module_system::sources::program_sources(&root, &entry)
        .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot read {}: {}", root.display(), e)))?;
    let module_path = match sources.iter().find(|s| same_file(&s.file, &file)) {
        Some(s) => s.module_path.clone(),
        None => {
            // test sources are not modules of the program: mount the file at its path under the root
            let mut path = if file.starts_with(&root) {
                crate::resolve::module_path_from_file(&root, &file)
            } else {
                file.file_stem().map(|s| vec![s.to_string_lossy().to_string()]).unwrap_or_default()
            };
            if path.is_empty() || sources.iter().any(|s| s.module_path == path) {
                path = vec![file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()];
            }
            sources.push(SourceFile { module_path: path.clone(), file: file.clone() });
            path
        }
    };
    Ok((super::project::parse_files(&sources)?, module_path, entry))
}

fn same_file(a: &Path, b: &Path) -> bool {
    a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(x, y)| x == y)
}

impl Interpreter {
    /// Load a program the way `wj run` sees it: a `main.wj` (or declared binary) owns its source tree and
    /// every module in it is loaded (like the compiled build); a loose script loads what it `use`s.
    /// Names are bound by the compiler's resolver either way (`project.rs`).
    pub fn load_program_at(&mut self, entry: &Path) -> Result<(), RuntimeError> {
        let files = program_files(entry)?;
        let deps = super::deps::dependency_sources(entry)?;
        self.load_sources(files, deps.windjammer, deps.rust)
    }

    /// Load the program a test file runs in (see [`test_program_files`]); returns the file's module path.
    pub fn load_test_program(&mut self, file: &Path) -> Result<Vec<String>, RuntimeError> {
        let (files, module_path, entry) = test_program_files(file)?;
        let deps = super::deps::dependency_sources(&entry)?;
        self.load_sources(files, deps.windjammer, deps.rust)?;
        Ok(module_path)
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
        if self.project_loading || self.missing_modules.contains(&(from, name.to_string())) {
            // whole-program loads know every file up front (module_system::sources)
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
        if self.project_loading {
            // bound later, all at once, by the shared resolver (project.rs)
            return Ok(());
        }
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
            // `use a::name::name`: when the module already defines an item called `name`, the last
            // segment is that item, not the module file `name.wj` found again by the search path.
            let item_shadows_module = last
                && !glob
                && i > start
                && (self.modules[cur].funcs.contains_key(&seg) || self.modules[cur].consts.contains_key(&seg));
            // Is it a module?
            let child = if item_shadows_module {
                None
            } else if cur == 0 && i == start && self.modules[0].children.get(&seg).is_none() {
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
