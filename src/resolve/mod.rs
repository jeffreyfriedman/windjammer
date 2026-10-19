//! Identity-based name resolution (shadow mode). See `MODULE.md`.

mod collect;
mod def;
mod lookup;
mod paths;
mod prelude;
mod res;
mod uses;
mod walk;

pub use collect::{Collected, ModuleScope};
pub use uses::{same_crate_callee_key, Binding, UseDecl};
pub use def::{Def, DefId, DefKind, DefTable};
pub use lookup::{Found, Ns};
pub use res::{
    CallKind, CallResolution, Diagnostic, DiagnosticKind, ExprKey, FnRef, Reason, Res, ResolveReport,
};

use crate::parser::ast::core::Program;
use std::path::{Path, PathBuf};

/// One parsed source file placed in the module tree.
pub struct ParsedFile<'a> {
    /// Module path of this file from the program root (`[]` for the root file).
    pub module_path: Vec<String>,
    pub file: PathBuf,
    pub program: &'a Program<'a>,
}

impl<'a> ParsedFile<'a> {
    pub fn new(module_path: Vec<String>, file: PathBuf, program: &'a Program<'a>) -> Self {
        Self {
            module_path,
            file,
            program,
        }
    }
}

/// Derive a module path from a file path relative to the source root:
/// `a/b.wj` -> `[a, b]`, `a/mod.wj` -> `[a]`, root `main.wj`/`lib.wj`/`mod.wj` -> `[]`.
pub fn module_path_from_file(root: &Path, file: &Path) -> Vec<String> {
    let rel = file.strip_prefix(root).unwrap_or(file).with_extension("");
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let last_is_root_name = parts
        .last()
        .is_some_and(|l| l == "mod" || l == "main" || l == "lib");
    if parts.last().is_some_and(|l| l == "mod") || (parts.len() == 1 && last_is_root_name) {
        parts.pop();
    }
    parts
}

/// The resolver's complete output.
pub struct ResolvedProgram<'a> {
    pub collected: Collected<'a>,
    pub calls: Vec<CallResolution>,
    /// Functions named as values (see [`FnRef`]).
    pub fn_refs: Vec<FnRef>,
    pub report: ResolveReport,
}

impl<'a> ResolvedProgram<'a> {
    pub fn defs(&self) -> &DefTable {
        &self.collected.defs
    }

    /// Calls whose enclosing function has the given full path.
    pub fn calls_in(&self, fn_path: &str) -> Vec<&CallResolution> {
        let ids = self.collected.defs.find_path(fn_path);
        self.calls
            .iter()
            .filter(|c| c.enclosing.is_some_and(|e| ids.contains(&e)))
            .collect()
    }
}

/// Resolve every call in a multi-file program. Pure: no IO, no global state.
pub fn resolve_program<'a>(files: &[ParsedFile<'a>]) -> ResolvedProgram<'a> {
    let mut collected = collect::collect(files);
    collected.link_impls();
    check_imports(&mut collected);
    let (calls, fn_refs) = walk::resolve_bodies(&collected);
    let report = ResolveReport::from_calls(&calls, std::mem::take(&mut collected.diagnostics));
    ResolvedProgram {
        collected,
        calls,
        fn_refs,
        report,
    }
}

/// Emit a `BadImport` diagnostic for imports that start inside the program but dead-end.
fn check_imports(c: &mut Collected<'_>) {
    let mut diags = Vec::new();
    for (module, scope) in &c.modules {
        for u in &scope.uses {
            if !import_is_bad(c, *module, u) {
                continue;
            }
            diags.push(Diagnostic {
                kind: DiagnosticKind::BadImport,
                message: format!("unresolved import `{}`", u.segments.join("::")),
                location: u.location.clone(),
            });
        }
    }
    c.diagnostics.extend(diags);
}

fn import_is_bad(c: &Collected<'_>, module: DefId, u: &uses::UseDecl) -> bool {
    c.resolve_type_path(module, &u.segments).is_empty()
        && c.lookup_path_value(module, &u.segments).is_empty()
}
