//! IR Pipeline Integration
//!
//! Entry point for the safety-typed IR compilation path. The pipeline runs
//! the unified constraint solver plus the three domain solvers (Effect, Taint,
//! Execution) on every compilation, producing an `IrModule` with resolved
//! `SafetyType` data.
//!
//! # Usage
//!
//! ```rust,ignore
//! use windjammer::ir::pipeline::IrPipeline;
//!
//! let pipeline = IrPipeline::new();
//! let ir_module = pipeline.lower_to_ir(&analyzed_functions, &registry);
//! ```

use super::{
    constraint_gen, EffectSolver, ExecutionValidator, IrFunction, Solver, TaintSolver,
};
use super::constraints::Constraint;
use super::effects::{self, EffectConstraint};
use super::safety_type::{BaseType, EffectSet, OwnedType};
use super::taint;
use std::collections::HashMap;

/// Configuration for the IR compilation pipeline.
#[derive(Debug, Clone)]
pub struct IrPipelineConfig {
    pub enable_effect_inference: bool,
    pub enable_taint_tracking: bool,
    pub enable_execution_modes: bool,
    pub enable_numeric_unification: bool,
    pub target: CompilationTarget,
}

impl Default for IrPipelineConfig {
    fn default() -> Self {
        Self {
            enable_effect_inference: true,
            enable_taint_tracking: true,
            enable_execution_modes: true,
            enable_numeric_unification: true,
            target: CompilationTarget::Rust,
        }
    }
}

/// Target backend for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTarget {
    Rust,
    Go,
    JavaScript,
}

/// The IR compilation pipeline.
///
/// Replaces the legacy `AnalyzedFunction` → codegen path with:
/// AST → Constraint Collection → Unified Solving → IR → Codegen
pub struct IrPipeline {
    config: IrPipelineConfig,
}

/// Result of lowering a module to IR.
#[derive(Debug)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub diagnostics: Vec<IrDiagnostic>,
    /// Per-function effect sets from the EffectSolver (empty until WJ-SEC-01 populates).
    pub effect_sets: HashMap<String, EffectSet>,
    /// Taint analysis results (empty until WJ-SEC-02 populates).
    pub taint_errors: Vec<String>,
    /// Execution mode validation results (empty until WJ-CONC-01 populates).
    pub execution_errors: Vec<String>,
    pub execution_warnings: Vec<String>,
}

/// Diagnostic emitted during IR lowering or solving.
#[derive(Debug)]
pub struct IrDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub span: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl IrPipeline {
    pub fn new() -> Self {
        Self::with_config(IrPipelineConfig::default())
    }

    pub fn with_config(config: IrPipelineConfig) -> Self {
        Self { config }
    }

    /// Lower analyzed functions to IR form using the unified constraint solver
    /// and the three domain solvers (Effect, Taint, Execution).
    ///
    /// Pipeline stages:
    /// 1. Convert `AnalyzedFunction`s to `IrFunction`s (lossless bridge)
    /// 2. Generate constraints and solve per-function (unified solver)
    /// 3. Run EffectSolver with stdlib declarations + forwarded constraints
    /// 4. Run TaintSolver with stdlib sources + forwarded constraints
    /// 5. Run ExecutionValidator with effect results fed in
    /// 6. Produce `IrModule` with resolved `SafetyType`s and domain results
    pub fn lower_to_ir(
        &mut self,
        analyzed: &[crate::analyzer::AnalyzedFunction],
        _registry: &crate::analyzer::SignatureRegistry,
    ) -> IrModule {
        let mut diagnostics = Vec::new();

        // Stage 1: Convert AnalyzedFunctions to IrFunctions (lossless bridge)
        let mut functions: Vec<IrFunction> =
            analyzed.iter().map(IrFunction::from_analyzed).collect();

        diagnostics.push(IrDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: format!("IR: lowered {} functions from analyzer", functions.len()),
            span: None,
        });

        // Stage 2: Generate constraints and solve per-function
        let mut total_constraints = 0usize;
        let mut total_solver_diags = 0usize;
        let mut effect_constraints_forwarded = Vec::new();
        let taint_constraints_forwarded: Vec<taint::TaintConstraint> = Vec::new();

        for (i, af) in analyzed.iter().enumerate() {
            let fc = constraint_gen::generate_constraints(af);
            total_constraints += fc.constraints.len();

            // Collect effect/taint constraints before solving (forward to domain solvers)
            for c in fc.constraints.iter() {
                match c {
                    Constraint::HasEffects(_, ref eff_set) => {
                        for eff in eff_set.iter() {
                            effect_constraints_forwarded.push(EffectConstraint::Performs {
                                function: fc.function_name.clone(),
                                effect: eff.clone(),
                            });
                        }
                    }
                    Constraint::EffectsUnion(_, ref deps) => {
                        for dep_var in deps {
                            // Map callee var back to function name via var_map
                            for (name, &var) in &fc.var_map.params {
                                if var == *dep_var {
                                    effect_constraints_forwarded.push(
                                        EffectConstraint::CallsInto {
                                            caller: fc.function_name.clone(),
                                            callee: name.clone(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            let solver = Solver::new(&fc.constraints);
            let result = solver.solve(&fc.constraints);

            total_solver_diags += result.diagnostics.len();
            for diag in &result.diagnostics {
                diagnostics.push(IrDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: format!("[{}] {}", fc.function_name, diag.message),
                    span: None,
                });
            }

            // Write solver results back to IrFunction param SafetyTypes
            if let Some(ir_fn) = functions.get_mut(i) {
                for (param_name, &var) in &fc.var_map.params {
                    if let Some(st) = ir_fn.param_types.get_mut(param_name) {
                        let root_idx = var.0 as usize;
                        if let Some(Some(ref base)) = result.types.get(root_idx) {
                            if *base != BaseType::Inferred {
                                st.base = base.clone();
                            }
                        }
                        if let Some(Some(ref own)) = result.ownership.get(root_idx) {
                            if *own != OwnedType::Inferred {
                                st.ownership = own.clone();
                            }
                        }
                    }
                }

                let ret_idx = fc.var_map.return_var.0 as usize;
                if let Some(Some(ref base)) = result.types.get(ret_idx) {
                    if *base != BaseType::Inferred {
                        ir_fn.return_type.base = base.clone();
                    }
                }
            }
        }

        diagnostics.push(IrDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: format!(
                "Solver: {} constraints across {} functions, {} diagnostics",
                total_constraints,
                functions.len(),
                total_solver_diags,
            ),
            span: None,
        });

        // Stage 3: EffectSolver — load stdlib + forwarded constraints
        let effect_result = if self.config.enable_effect_inference {
            let mut effect_solver = EffectSolver::new();
            for c in effects::stdlib_effect_declarations() {
                effect_solver.add_constraint(c);
            }
            for c in effect_constraints_forwarded {
                effect_solver.add_constraint(c);
            }
            let result = effect_solver.solve();
            if !result.errors.is_empty() {
                for err in &result.errors {
                    diagnostics.push(IrDiagnostic {
                        severity: DiagnosticSeverity::Warning,
                        message: format!("Effect: {}", err.message),
                        span: None,
                    });
                }
            }
            Some(result)
        } else {
            None
        };

        // Stage 4: TaintSolver — load stdlib sources + forwarded constraints
        let taint_result = if self.config.enable_taint_tracking {
            let mut taint_solver = TaintSolver::new();
            for c in taint::stdlib_taint_sources() {
                taint_solver.add_constraint(c);
            }
            for c in taint_constraints_forwarded {
                taint_solver.add_constraint(c);
            }
            let result = taint_solver.solve();
            if !result.errors.is_empty() {
                for err in &result.errors {
                    diagnostics.push(IrDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: format!("Taint: {}", err.message),
                        span: None,
                    });
                }
            }
            Some(result)
        } else {
            None
        };

        // Stage 5: ExecutionValidator — feed effect results in
        let exec_result = if self.config.enable_execution_modes {
            let mut exec_validator = ExecutionValidator::new();
            // Feed effect results so ExecutionValidator can warn about spawning pure functions
            if let Some(ref eff_result) = effect_result {
                exec_validator.set_function_effects(eff_result.resolved.clone());
            }
            // No execution constraints collected yet (WJ-CONC-01 will parse `async`/`spawn` keywords)
            let result = exec_validator.validate();
            if !result.errors.is_empty() {
                for err in &result.errors {
                    diagnostics.push(IrDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: format!("Execution: {}", err.message),
                        span: None,
                    });
                }
            }
            for warn in &result.warnings {
                diagnostics.push(IrDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: format!("Execution: {}", warn.message),
                    span: None,
                });
            }
            Some(result)
        } else {
            None
        };

        // Stage 6: Collect results into IrModule
        let effect_sets = effect_result
            .as_ref()
            .map(|r| r.resolved.clone())
            .unwrap_or_default();

        let taint_errors: Vec<String> = taint_result
            .as_ref()
            .map(|r| r.errors.iter().map(|e| e.message.clone()).collect())
            .unwrap_or_default();

        let execution_errors: Vec<String> = exec_result
            .as_ref()
            .map(|r| r.errors.iter().map(|e| e.message.clone()).collect())
            .unwrap_or_default();

        let execution_warnings: Vec<String> = exec_result
            .as_ref()
            .map(|r| r.warnings.iter().map(|w| w.message.clone()).collect())
            .unwrap_or_default();

        // Emit summary diagnostic
        let mut_count = functions
            .iter()
            .filter(|f| !f.mutated_params.is_empty())
            .count();
        let str_opt_count = functions
            .iter()
            .filter(|f| !f.str_ref_params.is_empty())
            .count();
        diagnostics.push(IrDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: format!(
                "IR summary: {} functions, {} with mut params, {} with str_ref optimizations, {} effect sets, {} taint errors, {} exec errors",
                functions.len(),
                mut_count,
                str_opt_count,
                effect_sets.len(),
                taint_errors.len(),
                execution_errors.len(),
            ),
            span: None,
        });

        IrModule {
            functions,
            diagnostics,
            effect_sets,
            taint_errors,
            execution_errors,
            execution_warnings,
        }
    }

    /// Lower multiple files to IR in library/multipass mode.
    ///
    /// Each entry is `(filename, analyzed_functions)`. All files share the
    /// global `registry` for cross-file signature resolution.
    ///
    /// Returns an `IrModuleSet` aggregating per-file `IrModule`s.
    pub fn lower_multi_file_to_ir(
        &mut self,
        files: &[(&str, &[crate::analyzer::AnalyzedFunction])],
        registry: &crate::analyzer::SignatureRegistry,
    ) -> IrModuleSet {
        let mut modules = Vec::new();
        let mut total_functions = 0usize;
        let mut total_diagnostics = 0usize;

        for &(filename, analyzed) in files {
            let module = self.lower_to_ir(analyzed, registry);
            total_functions += module.functions.len();
            total_diagnostics += module.diagnostics.len();
            modules.push(FileIrModule {
                filename: filename.to_string(),
                module,
            });
        }

        IrModuleSet {
            files: modules,
            total_functions,
            total_diagnostics,
        }
    }

    /// Generate code from IR (future: replaces legacy codegen path).
    ///
    /// Currently returns None, signaling that the legacy codegen should be used.
    /// When this returns Some, the legacy path can be bypassed.
    pub fn try_codegen_from_ir(&self, _module: &IrModule) -> Option<String> {
        None
    }

    /// Check if the IR pipeline is ready to fully replace the legacy path.
    ///
    /// Returns true only when all IR cutover categories are enabled and validated.
    pub fn is_ready_for_cutover(&self) -> bool {
        let config = crate::codegen::rust::generator::IrCutoverConfig::from_env();
        config.all_enabled()
    }
}

/// A named IR module from a specific file.
#[derive(Debug)]
pub struct FileIrModule {
    pub filename: String,
    pub module: IrModule,
}

/// Collection of IR modules from a multi-file build.
#[derive(Debug)]
pub struct IrModuleSet {
    pub files: Vec<FileIrModule>,
    pub total_functions: usize,
    pub total_diagnostics: usize,
}

impl IrModuleSet {
    /// Iterate over all diagnostics from all files.
    pub fn all_diagnostics(&self) -> impl Iterator<Item = (&str, &IrDiagnostic)> {
        self.files.iter().flat_map(|f| {
            f.module
                .diagnostics
                .iter()
                .map(move |d| (f.filename.as_str(), d))
        })
    }

    /// Collect all effect sets across files (merged).
    pub fn merged_effect_sets(&self) -> HashMap<String, EffectSet> {
        let mut merged = HashMap::new();
        for file_module in &self.files {
            for (func, effects) in &file_module.module.effect_sets {
                merged
                    .entry(func.clone())
                    .and_modify(|e: &mut EffectSet| *e = e.union(effects))
                    .or_insert_with(|| effects.clone());
            }
        }
        merged
    }

    /// Check if any file has taint errors.
    pub fn has_taint_errors(&self) -> bool {
        self.files.iter().any(|f| !f.module.taint_errors.is_empty())
    }

    /// Check if any file has execution errors.
    pub fn has_execution_errors(&self) -> bool {
        self.files
            .iter()
            .any(|f| !f.module.execution_errors.is_empty())
    }
}

impl Default for IrPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to check if the IR feature is active at runtime.
/// Always returns true when compiled with the feature; this function exists
/// so that calling code doesn't need cfg attributes everywhere.
#[inline]
pub fn ir_pipeline_available() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_source(source: &str) -> (Vec<crate::analyzer::AnalyzedFunction<'static>>, crate::analyzer::SignatureRegistry) {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let leaked_program: &'static _ = Box::leak(Box::new(program));
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(leaked_program).expect("analyze");
        (analyzed, registry)
    }

    #[test]
    fn test_pipeline_runs_all_domain_solvers() {
        let (analyzed, registry) = analyze_source("pub fn hello() -> i32 { 42 }");
        let mut pipeline = IrPipeline::new();
        let module = pipeline.lower_to_ir(&analyzed, &registry);

        assert_eq!(module.functions.len(), 1);
        assert!(module.taint_errors.is_empty());
        assert!(module.execution_errors.is_empty());
        assert!(module.execution_warnings.is_empty());
        // Stdlib effect declarations should be loaded
        assert!(!module.effect_sets.is_empty(), "stdlib effects should be populated");
    }

    #[test]
    fn test_pipeline_with_disabled_solvers() {
        let (analyzed, registry) = analyze_source("pub fn noop() {}");
        let config = IrPipelineConfig {
            enable_effect_inference: false,
            enable_taint_tracking: false,
            enable_execution_modes: false,
            enable_numeric_unification: true,
            target: CompilationTarget::Rust,
        };
        let mut pipeline = IrPipeline::with_config(config);
        let module = pipeline.lower_to_ir(&analyzed, &registry);

        assert_eq!(module.functions.len(), 1);
        assert!(module.effect_sets.is_empty());
        assert!(module.taint_errors.is_empty());
        assert!(module.execution_errors.is_empty());
    }

    #[test]
    fn test_pipeline_diagnostics_include_solver_summary() {
        let (analyzed, registry) = analyze_source("pub fn add(x: i32, y: i32) -> i32 { x }");
        let mut pipeline = IrPipeline::new();
        let module = pipeline.lower_to_ir(&analyzed, &registry);

        let has_solver_summary = module.diagnostics.iter().any(|d| d.message.contains("Solver:"));
        assert!(has_solver_summary, "should have solver summary diagnostic");

        let has_ir_summary = module.diagnostics.iter().any(|d| d.message.contains("IR summary:"));
        assert!(has_ir_summary, "should have IR summary diagnostic");
    }

    #[test]
    fn test_pipeline_multiple_functions() {
        let source = r#"
pub fn compute(x: f64) -> f64 { x }
pub fn display(name: string) {}
"#;
        let (analyzed, registry) = analyze_source(source);
        let mut pipeline = IrPipeline::new();
        let module = pipeline.lower_to_ir(&analyzed, &registry);

        assert_eq!(module.functions.len(), 2);
        assert!(module.taint_errors.is_empty());
    }
}
