//! One analysis of a program, shared by every consumer of one `wj check` run (docs/design/shared-program-analysis.md).
//!
//! The check asks the analyzer for the same answer several times: the diagnostics phase, the Rust generator inside the
//! move prediction and the mutability facts each analyze every file with the stdlib signatures and infer its numeric
//! types. [`ProgramAnalysis`] computes each of those products once, keeps it behind an `Arc` under a key that names how
//! it was derived, and hands the same value to the next consumer that asks for the same derivation.
//!
//! The module touches no file, environment variable or cache directory: the driver chooses the [`Mode`] and passes the
//! program text in. See `MODULE.md` for the invariants (purity audit, panics, what a key must name).

mod keys;
mod verify;

pub use verify::mismatch;

pub use keys::{FileKey, Globals};

use crate::analyzer::{AnalyzedFunction, Analyzer, SignatureRegistry};
use crate::debug_env::{count, Stage};
use crate::ir::numeric_bridge::UnifiedNumericInference;
use crate::parser::Program;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// How a session treats a repeated request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Reuse: the second request for a derivation gets the first one's product.
    Shared,
    /// Every request computes for itself, as the check did before the session existed. The differential oracle (G1).
    Off,
    /// Reuse, and recompute on every reuse and panic when the two differ (G2).
    Verify,
}

/// The analyzer's answer for one file: analyzed functions, the registry, and the trait methods; or its error text.
pub struct Analysis {
    pub result: Result<Analyzed, String>,
}

/// What `Analyzer::analyze_program_with_global_signatures` returns on success.
pub struct Analyzed {
    pub functions: Vec<AnalyzedFunction<'static>>,
    pub registry: SignatureRegistry,
    pub trait_methods: HashMap<String, HashMap<String, AnalyzedFunction<'static>>>,
}

type Slot<T> = Arc<OnceLock<Arc<T>>>;

/// The products of one run. Cheap to create; computes nothing until asked.
pub struct ProgramAnalysis {
    mode: Mode,
    stdlib: OnceLock<Arc<SignatureRegistry>>,
    analyses: Mutex<HashMap<keys::AnalysisKey, Slot<Analysis>>>,
    numerics: Mutex<HashMap<FileKey, Slot<UnifiedNumericInference>>>,
}

impl ProgramAnalysis {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            stdlib: OnceLock::new(),
            analyses: Mutex::new(HashMap::new()),
            numerics: Mutex::new(HashMap::new()),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The stdlib signatures every first-pass analysis reads (`SignatureRegistry::new()`): copied once per session, then
    /// shared by reference instead of copied for each file.
    fn stdlib(&self) -> Arc<SignatureRegistry> {
        if self.mode == Mode::Off {
            count(Stage::RegistryCopy);
            return Arc::new(SignatureRegistry::stdlib().clone());
        }
        self.stdlib
            .get_or_init(|| {
                count(Stage::RegistryCopy);
                Arc::new(SignatureRegistry::stdlib().clone())
            })
            .clone()
    }

    /// A1: `file` analyzed with the stdlib signatures. A panic in the analyzer propagates to the caller, as it did when
    /// every consumer ran the analyzer itself, and stores nothing.
    pub fn first_pass(&self, file: &FileKey, program: &Program<'static>) -> Arc<Analysis> {
        let key = keys::AnalysisKey::first_pass(file);
        self.analysis(key, program, || self.stdlib())
    }

    /// A2: `file` analyzed with `globals`, the stdlib signatures merged with every first-pass registry of the program
    /// (`global`, which the caller built by merging the products named by `globals`).
    pub fn second_pass(
        &self,
        file: &FileKey,
        program: &Program<'static>,
        globals: Globals,
        global: &Arc<SignatureRegistry>,
    ) -> Arc<Analysis> {
        let key = keys::AnalysisKey::second_pass(file, globals);
        self.analysis(key, program, || global.clone())
    }

    fn analysis(
        &self,
        key: keys::AnalysisKey,
        program: &Program<'static>,
        globals: impl Fn() -> Arc<SignatureRegistry>,
    ) -> Arc<Analysis> {
        let compute = || {
            let mut analyzer = Analyzer::new();
            let result = analyzer
                .analyze_program_with_global_arc(program, &globals())
                .map(|(functions, registry, trait_methods)| Analyzed {
                    functions,
                    registry,
                    trait_methods,
                });
            Arc::new(Analysis { result })
        };
        let slot = self.slot(&self.analyses, key.clone());
        self.get(
            &slot,
            compute,
            |fresh, reused| verify::analysis(&key, fresh, reused),
            Stage::AnalyzerReused,
        )
    }

    /// N: numeric inference of `file`. Reads the `.wj.meta` sidecars of the program's imports, which do not change
    /// during a run.
    pub fn numeric(
        &self,
        file: &FileKey,
        program: &Program<'static>,
    ) -> Arc<UnifiedNumericInference> {
        let compute = || {
            let mut num = UnifiedNumericInference::new();
            num.infer_program(program);
            Arc::new(num)
        };
        let slot = self.slot(&self.numerics, file.clone());
        self.get(
            &slot,
            compute,
            |fresh, reused| verify::numeric(file, fresh, reused),
            Stage::NumericReused,
        )
    }

    fn slot<K: std::hash::Hash + Eq, T>(
        &self,
        map: &Mutex<HashMap<K, Slot<T>>>,
        key: K,
    ) -> Slot<T> {
        map.lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key)
            .or_default()
            .clone()
    }

    fn get<T>(
        &self,
        slot: &Slot<T>,
        compute: impl Fn() -> Arc<T>,
        verify: impl Fn(&T, &T),
        reused: Stage,
    ) -> Arc<T> {
        if self.mode == Mode::Off {
            return compute();
        }
        let mut computed = false;
        let product = slot
            .get_or_init(|| {
                computed = true;
                compute()
            })
            .clone();
        if !computed {
            count(reused);
            if self.mode == Mode::Verify {
                verify(&compute(), &product);
            }
        }
        product
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn products_are_send_and_sync() {
        fn both<T: Send + Sync>() {}
        both::<Analysis>();
        both::<UnifiedNumericInference>();
        both::<ProgramAnalysis>();
    }
}
