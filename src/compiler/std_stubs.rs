//! `std/<module>.wj` stub sources that take part in a library build, and the runtime contracts their
//! (empty-bodied) analysis must not override.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;

/// Stdlib stub files prepended to the sources of a build, plus the declared signatures captured before
/// convergence starts.
#[derive(Default)]
pub(crate) struct StdStubs {
    files: HashSet<PathBuf>,
    contracts: HashMap<String, FunctionSignature>,
}

impl StdStubs {
    /// Step 0 of a library build: find the stdlib modules the user files import and prepend their stub sources.
    /// Returns the imported module names alongside the stub set.
    pub(crate) fn prepend_sources(
        wj_files: &[PathBuf],
        stdlib_dir: Option<PathBuf>,
        sources: &mut Vec<(PathBuf, String)>,
    ) -> (HashSet<String>, Self) {
        let mut needed = HashSet::new();
        for file in wj_files {
            if let Ok(source) = std::fs::read_to_string(file) {
                needed.extend(crate::compiler::library_copy_registry::stdlib_modules_from_source(&source));
            }
        }
        let mut stubs = Self::default();
        if let Some(dir) = stdlib_dir.filter(|_| !needed.is_empty()) {
            for module in &needed {
                let file = dir.join(format!("{module}.wj"));
                if let Ok(source) = std::fs::read_to_string(&file) {
                    stubs.files.insert(file.clone());
                    sources.push((file, source));
                }
            }
        }
        (needed, stubs)
    }

    /// Snapshot the declared signatures (the runtime-facing contract) before the convergence rounds rewrite them.
    pub(crate) fn capture_contracts(&mut self, registry: &SignatureRegistry) {
        if !self.files.is_empty() {
            self.contracts = registry.signatures.clone();
        }
    }

    /// Stubs feed signature convergence, so a rebuild re-analyses them exactly like a clean build does.
    pub(crate) fn mark_reanalysis(&self, sources: &[(PathBuf, String)], reanalysis: &mut HashSet<usize>) {
        for (i, (file, _)) in sources.iter().enumerate() {
            if self.files.contains(file) {
                reanalysis.insert(i);
            }
        }
    }

    /// The signature to merge for `name` analysed in `file`. A stub body never uses its parameters, so analysis
    /// infers `Borrowed` for a free function's `address: string` that the runtime takes by value
    /// (`http::server_serve`): the declared owned formal is kept. Methods keep the analysed receiver mode
    /// (builder methods consume `self`).
    pub(crate) fn effective_sig<'a>(
        &self,
        file: &Path,
        file_stem: &str,
        name: &str,
        sig: &'a FunctionSignature,
    ) -> Cow<'a, FunctionSignature> {
        if !self.files.contains(file) || name.contains("::") || sig.has_self_receiver {
            return Cow::Borrowed(sig);
        }
        let Some(declared) = self.contracts.get(&format!("{file_stem}::{name}")) else {
            return Cow::Borrowed(sig);
        };
        if declared.param_ownership.len() != sig.param_ownership.len() {
            return Cow::Borrowed(sig);
        }
        let mut out = sig.clone();
        for (i, mode) in declared.param_ownership.iter().enumerate() {
            let formal_owned = declared
                .param_types
                .get(i)
                .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)));
            if *mode == OwnershipMode::Owned && formal_owned && out.param_ownership[i] == OwnershipMode::Borrowed {
                out.param_ownership[i] = OwnershipMode::Owned;
                if let (Some(t), Some(d)) = (out.param_types.get_mut(i), declared.param_types.get(i)) {
                    *t = d.clone();
                }
            }
        }
        Cow::Owned(out)
    }
}
