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
    pub problems: Vec<SurfaceProblem>,
}

impl Surface {
    pub fn get(&self, name: &str) -> Option<&'static FunctionDecl<'static>> {
        self.fns.iter().copied().find(|f| f.name == name)
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

    pub fn is_rust_crate(&self, name: &str) -> bool {
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

/// Every surface file in the project at `manifest_dir` (inputs of `wj check`'s result cache), sorted.
pub fn surface_files(manifest_dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(manifest_dir.join(SURFACE_DIR)) else { return Vec::new() };
    let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == SURFACE_EXT)).collect();
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
            let has_surface = surface_path(manifest_dir, &key).is_file();
            if has_surface || is_rust_dependency(manifest_dir, spec) {
                names.push(key);
            }
        }
    }
    names.sort();
    names.dedup();
    let surfaces = names
        .iter()
        .filter_map(|n| {
            let path = surface_path(manifest_dir, n);
            let text = std::fs::read_to_string(&path).ok()?;
            Some(parse_surface(n, path, text))
        })
        .collect();
    RustCrates { names, surfaces }
}

/// Registry entries are Rust crates; `path` entries are Rust unless they hold a Windjammer package;
/// `git` and Windjammer-registry entries are not known to be Rust without fetching them.
fn is_rust_dependency(manifest_dir: &Path, spec: &toml::Value) -> bool {
    let Some(t) = spec.as_table() else { return spec.is_str() };
    if t.contains_key("git") || t.contains_key("registry") {
        return false;
    }
    match t.get("path").and_then(|p| p.as_str()) {
        Some(p) => {
            let dir = manifest_dir.join(p);
            !(dir.join("wj.toml").is_file() || dir.join("lib.wj").is_file() || dir.join("src").join("lib.wj").is_file())
        }
        None => true,
    }
}

/// Parse and validate one surface.
pub fn parse_surface(crate_name: &str, path: PathBuf, text: String) -> Surface {
    let mut problems = Vec::new();
    let mut fns = Vec::new();
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
                    _ => problems.push(SurfaceProblem {
                        line: 0,
                        message: "a surface may only contain `fn` declarations (ADR-0026)".to_string(),
                    }),
                }
            }
        }
        Err(e) => problems.push(SurfaceProblem { line: 0, message: format!("cannot parse the surface: {e}") }),
    }
    Surface { crate_name: crate_name.to_string(), path, text, fns, problems }
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
        std::fs::write(
            root.join("wj.toml"),
            "[dependencies]\nstr-sim = \"0.11\"\nhex = { version = \"0.4\" }\nwjpkg = { path = \"wjpkg\" }\nrs = { path = \"rs\" }\nremote = { git = \"https://example.invalid/x\" }\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("rust")).unwrap();
        std::fs::write(root.join("rust/hex.wji"), "pub fn encode(s: string) -> string\n").unwrap();
        let c = load(root);
        assert_eq!(c.names, ["hex", "rs", "str_sim"]);
        assert_eq!(c.surfaces.len(), 1);
        assert!(c.surface("hex").unwrap().get("encode").is_some());
        assert!(c.is_rust_crate("str_sim") && !c.is_rust_crate("wjpkg") && !c.is_rust_crate("remote"));
    }
}
