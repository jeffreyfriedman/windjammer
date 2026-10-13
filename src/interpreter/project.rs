//! Whole-program loading through the compiler's own module system and resolver.
//!
//! Files come from `module_system::sources` (the compiler's walker and module paths); every `use` is
//! bound by `resolve` (`DefId`s), the same resolver the compiler runs, so a name means the same thing
//! interpreted and compiled. Visibility (`pub`) is checked on the resolved definitions with Rust's rule
//! (a private item is visible in its own module and that module's descendants) and reported with
//! rustc's error code and wording.

use super::engine::*;
use super::error::{ErrorKind, RuntimeError, Span};
use super::loader::parse_source;
use crate::module_system::sources::SourceFile;
use crate::parser::{Item, Program};
use crate::resolve::{CallKind, Collected, Def, DefId, DefKind, Found, Res, ResolvedProgram};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A parsed program file placed in the module tree.
pub struct LoadedFile {
    pub module_path: Vec<String>,
    pub file: PathBuf,
    pub program: &'static Program<'static>,
}

/// Separately named library roots (dependencies) loaded next to the program: `(crate name, files)`.
/// Each is mounted as a top-level module and treated as its own crate for visibility.
pub type ExternalCrates = Vec<(String, Vec<SourceFile>)>;

fn semantic_at(msg: String, loc: Option<&crate::source_map::Location>) -> RuntimeError {
    let mut e = RuntimeError::new(ErrorKind::Semantic, msg);
    e.span = loc.map(|l| Span { file: l.file.to_string_lossy().to_string(), line: l.line, column: l.column });
    e
}

/// Parse every file (parse errors name the file, like the compiler's).
pub fn parse_files(files: &[SourceFile]) -> Result<Vec<LoadedFile>, RuntimeError> {
    files
        .iter()
        .map(|f| {
            let src = std::fs::read_to_string(&f.file).map_err(|e| {
                RuntimeError::new(ErrorKind::Semantic, format!("cannot read {}: {}", f.file.display(), e))
            })?;
            let program = parse_source(&src, &f.file.to_string_lossy()).map_err(|e| {
                RuntimeError::new(ErrorKind::Semantic, format!("parse error in {}: {}", f.file.display(), e))
            })?;
            Ok(LoadedFile {
                module_path: f.module_path.clone(),
                file: f.file.clone(),
                program: Box::leak(Box::new(program)),
            })
        })
        .collect()
}

/// Files reachable from a loose script through `use` / `mod` (a script does not own its directory).
pub fn script_sources(entry: &Path) -> Result<Vec<LoadedFile>, RuntimeError> {
    let root = entry.parent().map(Path::to_path_buf).unwrap_or_default();
    let first = parse_files(&[SourceFile { module_path: Vec::new(), file: entry.to_path_buf() }])?;
    let mut out: Vec<LoadedFile> = first;
    let mut seen: HashSet<Vec<String>> = HashSet::from([Vec::new()]);
    let mut i = 0;
    while i < out.len() {
        let base = out[i].module_path.clone();
        let mut wanted: Vec<Vec<String>> = Vec::new();
        referenced_modules(&out[i].program.items, &base, &mut wanted);
        for path in wanted {
            for k in 1..=path.len() {
                let prefix = path[..k].to_vec();
                if seen.contains(&prefix) {
                    continue;
                }
                let Some(file) = module_file(&root, &prefix) else {
                    if root.join(prefix.join("/")).is_dir() {
                        continue; // a directory module without mod.wj: keep walking
                    }
                    break;
                };
                seen.insert(prefix.clone());
                out.extend(parse_files(&[SourceFile { module_path: prefix, file }])?);
            }
        }
        i += 1;
    }
    Ok(out)
}

fn module_file(root: &Path, path: &[String]) -> Option<PathBuf> {
    let rel = path.join("/");
    [root.join(format!("{}.wj", rel)), root.join(&rel).join("mod.wj")].into_iter().find(|p| p.is_file())
}

/// Module paths a file's `use`/`mod` items may refer to, relative to the file's module and to the root.
fn referenced_modules(items: &[Item<'_>], base: &[String], out: &mut Vec<Vec<String>>) {
    for item in items {
        match item {
            Item::Use { path, .. } => {
                let mut segs: Vec<String> = Vec::new();
                for (i, s) in path.iter().enumerate() {
                    let s = s.split("::{").next().unwrap_or(s);
                    for part in s.split("::").filter(|p| !p.is_empty()) {
                        if i == 0 && segs.is_empty() {
                            if let Some(r) = part.strip_prefix("./") {
                                segs.push("self".into());
                                segs.push(r.into());
                                continue;
                            }
                            if part.starts_with("../") {
                                for _ in 0..part.matches("../").count() {
                                    segs.push("super".into());
                                }
                                let r = part.trim_start_matches("../");
                                if !r.is_empty() {
                                    segs.push(r.into());
                                }
                                continue;
                            }
                        }
                        segs.push(part.to_string());
                    }
                }
                let mut cur: Vec<String> = base.to_vec();
                let mut rel = segs.as_slice();
                let mut anchored = false;
                while let Some((h, t)) = rel.split_first() {
                    match h.as_str() {
                        "crate" => cur.clear(),
                        "self" => {}
                        "super" => {
                            cur.pop();
                        }
                        _ => break,
                    }
                    anchored = true;
                    rel = t;
                }
                if rel.first().is_some_and(|h| matches!(h.as_str(), "std" | "core" | "alloc")) {
                    continue;
                }
                let mut p = cur.clone();
                p.extend(rel.iter().filter(|s| *s != "*").cloned());
                out.push(p);
                if !anchored && !base.is_empty() {
                    out.push(rel.iter().filter(|s| *s != "*").cloned().collect());
                }
            }
            Item::Mod { name, items, .. } => {
                let mut p = base.to_vec();
                p.push(name.clone());
                if items.is_empty() {
                    out.push(p);
                } else {
                    referenced_modules(items, &p, out);
                }
            }
            _ => {}
        }
    }
}

/// Rust's privacy rule on resolved definitions.
struct Visibility<'c> {
    c: &'c Collected<'static>,
    /// Module defs that start another crate (dependencies): privacy does not leak across them.
    crate_roots: HashSet<DefId>,
}

impl Visibility<'_> {
    fn owning_module(&self, d: &Def) -> DefId {
        match d.kind {
            DefKind::Module => self.c.parent_of(d.id).unwrap_or(d.id),
            _ => d.module,
        }
    }

    fn is_ancestor_or_self(&self, anc: DefId, mut m: DefId) -> bool {
        loop {
            if m == anc {
                return true;
            }
            if self.crate_roots.contains(&m) {
                return false;
            }
            match self.c.parent_of(m) {
                Some(p) => m = p,
                None => return false,
            }
        }
    }

    fn crate_of(&self, mut m: DefId) -> DefId {
        loop {
            if self.crate_roots.contains(&m) {
                return m;
            }
            match self.c.parent_of(m) {
                Some(p) => m = p,
                None => return m,
            }
        }
    }

    /// Visible from `from`: public items everywhere (file modules are always `pub mod`), private ones only
    /// in their module's subtree within the same crate.
    fn visible(&self, d: &Def, from: DefId) -> bool {
        if d.is_pub || matches!(d.kind, DefKind::Variant { .. }) {
            return true;
        }
        let owner = self.owning_module(d);
        self.crate_of(owner) == self.crate_of(from) && self.is_ancestor_or_self(owner, from)
    }
}

/// rustc's noun for a definition kind (E0603 wording).
fn noun(kind: DefKind) -> &'static str {
    match kind {
        DefKind::Fn | DefKind::ExternFn => "function",
        DefKind::Struct => "struct",
        DefKind::Enum => "enum",
        DefKind::Trait => "trait",
        DefKind::Const => "constant",
        DefKind::Static => "static",
        DefKind::Module => "module",
        DefKind::TypeAlias => "type alias",
        DefKind::Method { .. } => "method",
        DefKind::AssocFn { .. } => "associated function",
        _ => "item",
    }
}

impl Interpreter {
    /// Load a program whose files are already known (project mode) and bind it through the resolver.
    /// `rust_crates`: names of Rust-crate dependencies (calls into them are reported by the preflight).
    pub fn load_sources(
        &mut self,
        files: Vec<LoadedFile>,
        externals: Vec<(String, Vec<LoadedFile>)>,
        rust_crates: Vec<String>,
    ) -> Result<(), RuntimeError> {
        let mut all: Vec<LoadedFile> = files;
        let mut crate_paths: Vec<Vec<String>> = Vec::new();
        for (name, lib) in externals {
            if all.iter().any(|f| f.module_path.first() == Some(&name)) {
                return Err(RuntimeError::new(
                    ErrorKind::Semantic,
                    format!("dependency `{}` has the same name as a module of this program", name),
                ));
            }
            crate_paths.push(vec![name.clone()]);
            for mut f in lib {
                f.module_path.insert(0, name.clone());
                all.push(f);
            }
        }
        let entry = &all[0];
        self.modules[0].file = entry.file.clone();
        self.modules[0].name = entry.file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut ids: Vec<usize> = Vec::with_capacity(all.len());
        for f in &all {
            ids.push(self.ensure_module_path(&f.module_path, &f.file));
        }
        self.project_loading = true;
        let reg: Result<(), RuntimeError> = all.iter().zip(&ids).try_for_each(|(f, &m)| self.register_items(m, &f.program.items));
        self.project_loading = false;
        reg?;
        let inputs: Vec<crate::resolve::ParsedFile<'static>> = all
            .iter()
            .map(|f| crate::resolve::ParsedFile::new(f.module_path.clone(), f.file.clone(), f.program))
            .collect();
        let resolved = crate::resolve::resolve_program(&inputs);
        self.reject_shadowed_type_names(&resolved)?;
        let module_ids = self.module_ids(&resolved.collected);
        let crate_roots: HashSet<DefId> = crate_paths
            .iter()
            .filter_map(|p| resolved.collected.defs.find_path(&p.join("::")).iter().copied().find(|d| resolved.collected.defs.get(*d).kind == DefKind::Module))
            .collect();
        let vis = Visibility { c: &resolved.collected, crate_roots };
        self.bind_uses(&resolved, &module_ids, &vis)?;
        check_call_privacy(&resolved, &vis)?;
        let bridged = |name: &str| self.has_host_fn(name);
        let std_fn = |path: &str| self.builtins.has_function(path);
        super::preflight::check(&all, &resolved, &rust_crates, &bridged, &std_fn)?;
        self.finish_registration()
    }

    /// Module index for a module path, creating directory/file modules on the way.
    fn ensure_module_path(&mut self, path: &[String], file: &Path) -> usize {
        let mut cur = 0;
        for (i, seg) in path.iter().enumerate() {
            cur = match self.modules[cur].children.get(seg) {
                Some(&c) => c,
                None => {
                    let dir = self.modules[cur].file.parent().map(Path::to_path_buf).unwrap_or_default();
                    let placeholder = if i + 1 == path.len() { file.to_path_buf() } else { dir.join(seg).join("mod.wj") };
                    let c = self.new_module(seg, placeholder, Some(cur));
                    self.modules[cur].children.insert(seg.clone(), c);
                    c
                }
            };
        }
        if !path.is_empty() {
            self.modules[cur].file = file.to_path_buf();
        }
        cur
    }

    /// Interpreter module index of every resolver module def (matched by module path).
    fn module_ids(&self, c: &Collected<'static>) -> HashMap<DefId, usize> {
        let mut by_path: HashMap<String, usize> = HashMap::new();
        let mut stack: Vec<(usize, String)> = vec![(0, String::new())];
        while let Some((m, p)) = stack.pop() {
            for (name, &child) in &self.modules[m].children {
                let cp = if p.is_empty() { name.clone() } else { format!("{}::{}", p, name) };
                stack.push((child, cp));
            }
            by_path.insert(p, m);
        }
        c.defs
            .iter()
            .filter(|d| d.kind == DefKind::Module)
            .filter_map(|d| by_path.get(&d.path).map(|&m| (d.id, m)))
            .collect()
    }

    /// Types are keyed by name inside the interpreter; refuse (loudly) programs where that would merge two types.
    fn reject_shadowed_type_names(&self, r: &ResolvedProgram<'static>) -> Result<(), RuntimeError> {
        let mut seen: HashMap<&str, &Def> = HashMap::new();
        for d in r.collected.defs.iter() {
            if !matches!(d.kind, DefKind::Struct | DefKind::Enum | DefKind::Trait) {
                continue;
            }
            if let Some(prev) = seen.get(d.name.as_str()) {
                if prev.module != d.module {
                    let m = |x: &Def| {
                        let p = r.collected.module_path_of(x.module);
                        if p.is_empty() { "crate".to_string() } else { p.to_string() }
                    };
                    let what = format!(
                        "{} `{}` is defined in two modules (`{}` and `{}`)",
                        noun(d.kind),
                        d.name,
                        m(prev),
                        m(d)
                    );
                    let (file, line, col) = d.location.as_ref().map(|l| (l.file.clone(), l.line, l.column)).unwrap_or_default();
                    let e = super::preflight::err("modules.same_name_types", what, &file, line, col);
                    return Err(e);
                }
            }
            seen.insert(&d.name, d);
        }
        Ok(())
    }

    fn import_for(&self, c: &Collected<'static>, ids: &HashMap<DefId, usize>, d: &Def) -> Option<Import> {
        match d.kind {
            DefKind::Module => ids.get(&d.id).map(|&m| Import::Module(m)),
            DefKind::Variant { owner } => {
                let e = c.defs.get(owner);
                ids.get(&e.module).map(|&m| Import::Item(m, format!("{}::{}", e.name, d.name)))
            }
            _ => ids.get(&d.module).map(|&m| Import::Item(m, d.name.clone())),
        }
    }

    fn bind_uses(&mut self, r: &ResolvedProgram<'static>, ids: &HashMap<DefId, usize>, vis: &Visibility<'_>) -> Result<(), RuntimeError> {
        let c = &r.collected;
        let mut scopes: Vec<(&DefId, &crate::resolve::ModuleScope)> = c.modules.iter().collect();
        scopes.sort_by_key(|(d, _)| **d);
        for (mdef, scope) in scopes {
            let Some(&m) = ids.get(mdef) else { continue };
            for u in &scope.uses {
                let segs = &u.segments;
                let mut found: Vec<Found> = c.resolve_type_path(*mdef, segs);
                found.extend(c.lookup_path_value(*mdef, segs));
                let defs: Vec<&Def> = found.iter().filter_map(|f| match f { Found::Def(d) => Some(c.defs.get(*d)), _ => None }).collect();
                if let Some(bad) = defs.iter().find(|d| !vis.visible(d, *mdef)) {
                    return Err(semantic_at(format!("{} `{}` is private (E0603)", noun(bad.kind), bad.name), u.location.as_ref()));
                }
                match &u.binding {
                    crate::resolve::Binding::Named(local) => {
                        // a module or type binding wins over a value of the same name (tuple-struct ctor)
                        let pick = defs.iter().find(|d| d.kind == DefKind::Module).or_else(|| defs.first());
                        let imp = match pick {
                            Some(d) => self.import_for(c, ids, d),
                            None => found.iter().find_map(|f| match f {
                                Found::External(p) => Some(Import::Std(p.join("::"))),
                                _ => None,
                            }),
                        };
                        if let Some(imp) = imp {
                            self.modules[m].imports.insert(local.clone(), imp);
                        } else if defs.is_empty() {
                            return Err(semantic_at(format!("unresolved import `{}` (E0432)", segs.join("::")), u.location.as_ref()));
                        }
                    }
                    crate::resolve::Binding::Glob => {
                        for d in defs.iter().filter(|d| matches!(d.kind, DefKind::Module | DefKind::Enum)) {
                            self.bind_glob(c, ids, vis, d, m, *mdef);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// `use target::*`: every name visible in `target` (items and its own imports); enum: its variants.
    fn bind_glob(&mut self, c: &Collected<'static>, ids: &HashMap<DefId, usize>, vis: &Visibility<'_>, target: &Def, into: usize, from: DefId) {
        let mut names: Vec<String> = Vec::new();
        if target.kind == DefKind::Enum {
            for v in c.members.get(&target.id).into_iter().flatten() {
                names.push(c.defs.get(*v).name.clone());
            }
        } else if let Some(scope) = c.modules.get(&target.id) {
            names.extend(scope.items.keys().cloned());
            for u in &scope.uses {
                if let crate::resolve::Binding::Named(n) = &u.binding {
                    if u.is_pub {
                        names.push(n.clone());
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        for n in names {
            let segs = [n.clone()];
            let mut found: Vec<DefId> = Vec::new();
            for f in c.lookup(target.id, &segs[0], crate::resolve::Ns::Type).into_iter().chain(c.lookup(target.id, &segs[0], crate::resolve::Ns::Value)) {
                if let Found::Def(d) = f {
                    found.push(d);
                }
            }
            if target.kind == DefKind::Enum {
                found = c.members.get(&target.id).into_iter().flatten().copied().filter(|v| c.defs.get(*v).name == n).collect();
            }
            let pick = found.iter().map(|d| c.defs.get(*d)).find(|d| vis.visible(d, from) && !matches!(d.kind, DefKind::Impl));
            if let Some(d) = pick {
                if let Some(imp) = self.import_for(c, ids, d) {
                    self.modules[into].imports.entry(n).or_insert(imp);
                }
            }
        }
    }
}

/// Qualified calls to private functions (`math::helper()`), checked on the resolver's call table.
fn check_call_privacy(r: &ResolvedProgram<'static>, vis: &Visibility<'_>) -> Result<(), RuntimeError> {
    for call in &r.calls {
        if call.kind != CallKind::Path && call.kind != CallKind::Free {
            continue;
        }
        let Res::Def(id) = &call.res else { continue };
        let d = r.collected.defs.get(*id);
        if !matches!(d.kind, DefKind::Fn | DefKind::ExternFn | DefKind::Const | DefKind::Static) {
            continue;
        }
        if !vis.visible(d, call.module) {
            return Err(semantic_at(format!("{} `{}` is private (E0603)", noun(d.kind), d.name), call.location.as_ref()));
        }
    }
    Ok(())
}
