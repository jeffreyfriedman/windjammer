//! Rust crate dependencies and their signature surfaces (ADR-0026).
//!
//! A Rust crate is a `[dependencies]` entry of `wj.toml` that is not a Windjammer package (same rule as
//! [`super::graph`]: registry entries, and `path` entries without `wj.toml`/`lib.wj`). Its surface is
//! `rust/<crate>.wji` next to `wj.toml`: body-less `fn` declarations typed in Windjammer types, with the
//! ADR-0021 effect decorators. The `.wji` extension keeps it out of the module walker: an interface is
//! not a module of the program.
//!
//! This module only loads and validates; the checker (`check::rust_calls`), the build (`compiler`, which
//! turns a surface into crate metadata) and the effect pass consume [`RustCrates`].

use crate::parser::ast::core::{FunctionDecl, Item, Program};
use std::path::{Path, PathBuf};

/// Directory (next to `wj.toml`) that holds the surfaces.
pub const SURFACE_DIR: &str = "rust";
/// Extension of a surface file.
pub const SURFACE_EXT: &str = "wji";

/// A problem in a surface file (reported as `WJ0050` at `line`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceProblem {
    pub line: usize,
    pub message: String,
}

/// One crate's declared surface.
pub struct Surface {
    /// Crate name as a path names it (`-` written `_`).
    pub crate_name: String,
    pub path: PathBuf,
    pub text: String,
    pub fns: Vec<&'static FunctionDecl<'static>>,
    /// Opaque crate types (`pub struct Regex {}`: no fields; values live in the crate, ADR-0031 handles).
    pub types: Vec<String>,
    /// Methods and associated functions of those types (`impl Regex { pub fn new(..) -> .. }`): `(type, decl)`.
    pub methods: Vec<(String, &'static FunctionDecl<'static>)>,
    pub problems: Vec<SurfaceProblem>,
    /// The generator's header when the file was generated (cached or `--write` copy).
    pub generated: Option<super::rust_surface_gen::Header>,
    /// Functions the generator listed as skipped, with the reason.
    pub skipped: Vec<(String, String)>,
}

impl Surface {
    /// A free function (`levenshtein`) or a method of an opaque type (`Regex::new`).
    pub fn get(&self, name: &str) -> Option<&'static FunctionDecl<'static>> {
        match name.split_once("::") {
            Some((ty, m)) => self.methods.iter().find(|(t, f)| t == ty && f.name == m).map(|(_, f)| *f),
            None => self.fns.iter().copied().find(|f| f.name == name),
        }
    }
}

/// The project's Rust crate dependencies and the surfaces that exist for them.
#[derive(Default)]
pub struct RustCrates {
    /// Every Rust crate dependency (path spelling, `-` written `_`), sorted.
    pub names: Vec<String>,
    pub surfaces: Vec<Surface>,
}

impl RustCrates {
    pub fn surface(&self, crate_name: &str) -> Option<&Surface> {
        self.surfaces.iter().find(|s| s.crate_name == crate_name)
    }

    pub(crate) fn is_rust_crate(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// The surface file of `crate_name` in the project at `manifest_dir`.
pub fn surface_path(manifest_dir: &Path, crate_name: &str) -> PathBuf {
    manifest_dir.join(SURFACE_DIR).join(format!("{crate_name}.{SURFACE_EXT}"))
}

/// Every surface file in the project at `manifest_dir`, hand-written and cached, plus the lock files their
/// staleness is judged against (inputs of `wj check`'s result cache), sorted.
pub fn surface_files(manifest_dir: &Path) -> Vec<PathBuf> {
    let is_surface = |p: &PathBuf| p.extension().is_some_and(|e| e == SURFACE_EXT);
    let mut v: Vec<PathBuf> = std::fs::read_dir(manifest_dir.join(SURFACE_DIR)).map(|rd| rd.flatten().map(|e| e.path()).filter(is_surface).collect()).unwrap_or_default();
    let cache = manifest_dir.join(".wj-cache").join(super::rust_surface_gen::cache::CACHE_SUBDIR);
    if let Ok(rd) = std::fs::read_dir(cache) {
        v.extend(rd.flatten().map(|e| e.path().join("surface.wji")).filter(|p| p.is_file()));
    }
    if !v.is_empty() {
        v.extend(super::rust_surface_gen::locate::lock_files(manifest_dir, None));
    }
    v.sort();
    v
}

/// Rust crates of the project whose `wj.toml` is in `manifest_dir`, with their surfaces.
/// No manifest, or one that does not parse: empty (the manifest's own errors are reported elsewhere).
pub fn load(manifest_dir: &Path) -> RustCrates {
    let Ok(text) = std::fs::read_to_string(manifest_dir.join("wj.toml")) else { return RustCrates::default() };
    let Ok(doc) = text.parse::<toml::Value>() else { return RustCrates::default() };
    let mut names: Vec<String> = Vec::new();
    if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
        for (name, spec) in deps {
            let key = name.replace('-', "_");
            // a hand-written surface marks a crate as Rust, unless the entry is a Windjammer package
            let has_surface = surface_path(manifest_dir, &key).is_file() && !is_windjammer_path(manifest_dir, spec);
            if has_surface || is_rust_dependency(manifest_dir, spec) {
                names.push(key);
            }
        }
    }
    names.sort();
    names.dedup();
    // precedence: a hand-written `rust/<crate>.wji` is source and wins over the generated cache copy
    let surfaces = names
        .iter()
        .filter_map(|n| {
            let hand = surface_path(manifest_dir, n);
            let path = if hand.is_file() { hand } else { super::rust_surface_gen::cache::surface_path(manifest_dir, n) };
            let text = std::fs::read_to_string(&path).ok()?;
            Some(parse_surface(n, path, text))
        })
        .collect();
    RustCrates { names, surfaces }
}

/// Registry entries are Rust crates; `path` entries are Rust unless they hold a Windjammer package, or are the
/// `build/` output of one (`super::graph::is_windjammer_path_dep`, the rule the dependency graph uses);
/// `git` and Windjammer-registry entries are not known to be Rust without fetching them.
fn is_rust_dependency(manifest_dir: &Path, spec: &toml::Value) -> bool {
    let Some(t) = spec.as_table() else { return spec.is_str() };
    if t.contains_key("git") || t.contains_key("registry") {
        return false;
    }
    match t.get("path").and_then(|p| p.as_str()) {
        Some(p) => !super::graph::is_windjammer_path_dep(&super::graph::dep_path(manifest_dir, p)),
        None => true,
    }
}

fn is_windjammer_path(manifest_dir: &Path, spec: &toml::Value) -> bool {
    spec.get("path").and_then(|p| p.as_str()).is_some_and(|p| super::graph::is_windjammer_path_dep(&super::graph::dep_path(manifest_dir, p)))
}

/// Parse and validate one surface.
pub fn parse_surface(crate_name: &str, path: PathBuf, text: String) -> Surface {
    let mut problems = Vec::new();
    let mut fns = Vec::new();
    let mut types: Vec<String> = Vec::new();
    let mut methods = Vec::new();
    let mut lexer = crate::lexer::Lexer::new(&text);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new_with_source(tokens, path.display().to_string(), text.clone());
    match parser.parse() {
        Ok(program) => {
            let program: &'static Program<'static> = Box::leak(Box::new(program));
            for item in &program.items {
                match item {
                    Item::Function { decl, location } => {
                        let line = location.as_ref().map(|l| l.line).unwrap_or(0);
                        if !decl.body.is_empty() {
                            problems.push(SurfaceProblem {
                                line,
                                message: format!("`fn {}` has a body: a surface only declares signatures (the implementation is the Rust crate)", decl.name),
                            });
                        } else if !decl.type_params.is_empty() {
                            problems.push(SurfaceProblem {
                                line,
                                message: format!("`fn {}` is generic: declare a concrete signature (generic crate APIs are not supported, ADR-0026)", decl.name),
                            });
                        } else {
                            fns.push(decl);
                        }
                    }
                    Item::Use { .. } => {}
                    Item::Struct { decl, location } if decl.fields.is_empty() && decl.tuple_fields.is_none() && decl.type_params.is_empty() => {
                        let _ = location;
                        types.push(decl.name.clone());
                    }
                    Item::Struct { decl, location } => problems.push(SurfaceProblem {
                        line: location.as_ref().map(|l| l.line).unwrap_or(0),
                        message: format!("`struct {}` has fields or type parameters: a surface declares an opaque crate type as `pub struct {} {{}}` (ADR-0031)", decl.name, decl.name),
                    }),
                    Item::Impl { block, location } if block.trait_name.is_none() && block.type_params.is_empty() => {
                        let line = location.as_ref().map(|l| l.line).unwrap_or(0);
                        for f in &block.functions {
                            if !f.body.is_empty() || !f.type_params.is_empty() {
                                problems.push(SurfaceProblem {
                                    line,
                                    message: format!("`{}::{}` has a body or is generic: a surface only declares concrete signatures", block.type_name, f.name),
                                });
                            } else {
                                methods.push((block.type_name.clone(), f));
                            }
                        }
                    }
                    _ => problems.push(SurfaceProblem {
                        line: 0,
                        message: "a surface may only contain `fn` declarations, opaque types (`pub struct T {}`) and their `impl` blocks (ADR-0026, ADR-0031)".to_string(),
                    }),
                }
            }
        }
        Err(e) => problems.push(SurfaceProblem { line: 0, message: format!("cannot parse the surface: {e}") }),
    }
    let generated = super::rust_surface_gen::header(&text);
    let skipped = if generated.is_some() { super::rust_surface_gen::skipped_in(&text) } else { Vec::new() };
    for (t, f) in &methods {
        if !types.contains(t) {
            problems.push(SurfaceProblem { line: 0, message: format!("`impl {t}` for `{t}::{}`: declare the type first (`pub struct {t} {{}}`)", f.name) });
        }
    }
    Surface { crate_name: crate_name.to_string(), path, text, fns, types, methods, problems, generated, skipped }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(text: &str) -> Surface {
        parse_surface("demo", PathBuf::from("rust/demo.wji"), text.to_string())
    }

    #[test]
    fn declarations_are_collected() {
        let s = surface("@pure\npub fn f(a: string) -> usize\npub fn g()\n");
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert_eq!(s.fns.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["f", "g"]);
        assert_eq!(s.get("f").unwrap().parameters.len(), 1);
        assert!(s.get("h").is_none());
    }

    #[test]
    fn opaque_types_and_their_methods_are_collected() {
        let s = surface("pub struct Counter {}\n\nimpl Counter {\n    pub fn new(start: i64) -> Counter\n    pub fn bump(&mut self) -> i64\n}\n\npub fn total(c: &Counter) -> i64\n");
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert_eq!(s.types, ["Counter"]);
        assert!(s.get("Counter::new").is_some() && s.get("Counter::bump").is_some() && s.get("total").is_some());
        assert!(s.get("Counter::nope").is_none() && s.get("bump").is_none());
        let bad = surface("pub struct P {\n    x: int,\n}\nimpl Q {\n    pub fn f()\n}\n");
        assert_eq!(bad.problems.len(), 2, "{:?}", bad.problems);
    }

    #[test]
    fn bodies_and_generics_are_rejected() {
        let s = surface("pub fn f() -> int {\n    1\n}\npub fn g<T>(x: T)\n");
        assert_eq!(s.problems.len(), 2, "{:?}", s.problems);
        assert!(s.problems[0].message.contains("has a body"));
        assert!(s.problems[1].message.contains("generic"));
        assert!(s.fns.is_empty());
    }

    #[test]
    fn manifest_classification() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("wjpkg")).unwrap();
        std::fs::write(root.join("wjpkg/wj.toml"), "[package]\nname = \"wjpkg\"\n").unwrap();
        std::fs::create_dir_all(root.join("rs")).unwrap();
        // `wjpkg/build` is the generated Rust of the Windjammer package: still Windjammer
        std::fs::create_dir_all(root.join("wjpkg/build/src")).unwrap();
        std::fs::write(root.join("wjpkg/build/Cargo.toml"), "[package]\nname = \"wjpkg\"\n").unwrap();
        std::fs::write(
            root.join("wj.toml"),
            "[dependencies]\nstr-sim = \"0.11\"\nhex = { version = \"0.4\" }\nwjpkg = { path = \"wjpkg\" }\nwjout = { path = \"wjpkg/build\" }\nrs = { path = \"rs\" }\nremote = { git = \"https://example.invalid/x\" }\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("rust")).unwrap();
        std::fs::write(root.join("rust/hex.wji"), "pub fn encode(s: string) -> string\n").unwrap();
        let c = load(root);
        assert_eq!(c.names, ["hex", "rs", "str_sim"]);
        assert_eq!(c.surfaces.len(), 1);
        assert!(c.surface("hex").unwrap().get("encode").is_some());
        assert!(c.is_rust_crate("str_sim") && !c.is_rust_crate("wjpkg") && !c.is_rust_crate("wjout") && !c.is_rust_crate("remote"));
    }
}
