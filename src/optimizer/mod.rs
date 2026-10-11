//! Post-codegen Rust optimizer. See `MODULE.md`.
//!
//! `run` parses the generated Rust with `syn`, lets each enabled pass report
//! span edits that are provably behaviour-preserving, applies them, and
//! re-parses the result (an edit set that yields unparsable Rust is discarded).

pub mod config;
mod edit;
mod facts;
mod fn_walk;
pub mod mutants;
mod passes;

pub use config::OptimizationConfig;
pub use mutants::Mutant;

use std::collections::BTreeMap;

/// Result of an optimizer run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationOutput {
    pub code: String,
    /// Applied edits per pass name.
    pub applied: BTreeMap<&'static str, usize>,
}

const MAX_ROUNDS: usize = 3;

/// Optimize generated Rust source. Unparsable input is returned unchanged.
pub fn run(config: &OptimizationConfig, rust_source: &str) -> OptimizationOutput {
    let mut code = rust_source.to_string();
    let mut applied: BTreeMap<&'static str, usize> = BTreeMap::new();
    if *config == OptimizationConfig::disabled() {
        return OptimizationOutput { code, applied };
    }
    for _ in 0..MAX_ROUNDS {
        let Ok(file) = syn::parse_file(&code) else { break };
        if declares_macros(&file) {
            break;
        }
        let idx = edit::SourceIndex::new(&code);
        let ctx = passes::Ctx { file: &file, idx: &idx, config };
        let edits = passes::collect(&ctx);
        let (next, done) = edit::apply(&code, edits);
        if done.is_empty() || syn::parse_file(&next).is_err() {
            break;
        }
        for e in &done {
            *applied.entry(e.pass).or_default() += 1;
        }
        code = next;
    }
    OptimizationOutput { code, applied }
}

/// Files defining `macro_rules!` may bind names invisibly to the fact collector.
fn declares_macros(file: &syn::File) -> bool {
    file.items.iter().any(|i| matches!(i, syn::Item::Macro(m) if m.mac.path.is_ident("macro_rules")))
}

/// Optimize with production defaults.
pub fn run_default(rust_source: &str) -> String {
    run(&OptimizationConfig::production(), rust_source).code
}

#[cfg(test)]
mod tests;
