//! IR Pipeline Integration
//!
//! Entry point for the safety-typed IR compilation path. When the `safety_typed_ir`
//! feature is enabled, this module provides an alternative compilation pipeline that
//! uses the unified constraint solver and IR nodes instead of the legacy
//! `AnalyzedFunction`-based approach.
//!
//! # Usage
//!
//! ```rust,ignore
//! #[cfg(feature = "safety_typed_ir")]
//! use windjammer::ir::pipeline::IrPipeline;
//!
//! let pipeline = IrPipeline::new();
//! let ir_module = pipeline.lower_to_ir(&analyzed_functions, &registry);
//! let rust_code = pipeline.codegen_from_ir(&ir_module);
//! ```

use super::{
    ConstraintSet, EffectSolver, ExecutionValidator, IrFunction, NumericSolver, Solver, TaintSolver,
};

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

    /// Lower analyzed functions to IR form using the unified constraint solver.
    ///
    /// This is the main entry point for the new pipeline. It:
    /// 1. Collects constraints from the AST/analyzed functions
    /// 2. Runs the unified solver to determine ownership, effects, taint
    /// 3. Produces IR nodes with resolved SafetyTypes
    pub fn lower_to_ir(
        &mut self,
        _analyzed: &[crate::analyzer::AnalyzedFunction],
        _registry: &crate::analyzer::SignatureRegistry,
    ) -> IrModule {
        let mut diagnostics = Vec::new();

        // Phase 1: Collect constraints from analyzed functions
        // (Currently a no-op bridge — future work will extract constraints from AST)
        let constraints = ConstraintSet::new();

        // Phase 2: Run unified solver
        let solver = Solver::new(&constraints);
        let _solver_result = solver.solve(&constraints);

        // Phase 3: Run effect inference
        if self.config.enable_effect_inference {
            let effect_solver = EffectSolver::default();
            let _effect_result = effect_solver.solve();
        }

        // Phase 4: Run taint tracking
        if self.config.enable_taint_tracking {
            let taint_solver = TaintSolver::default();
            let _taint_result = taint_solver.solve();
        }

        // Phase 5: Validate execution modes
        if self.config.enable_execution_modes {
            let execution_validator = ExecutionValidator::default();
            let _exec_result = execution_validator.validate();
        }

        // Phase 6: Run numeric unification
        if self.config.enable_numeric_unification {
            let numeric_solver = NumericSolver::default();
            let _numeric_result = numeric_solver.solve();
        }

        diagnostics.push(IrDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: "IR pipeline executed successfully (bridge mode)".to_string(),
            span: None,
        });

        IrModule {
            functions: Vec::new(),
            diagnostics,
        }
    }

    /// Generate code from IR (future: replaces legacy codegen path).
    ///
    /// Currently returns None, signaling that the legacy codegen should be used.
    /// When this returns Some, the legacy path can be bypassed.
    pub fn try_codegen_from_ir(&self, _module: &IrModule) -> Option<String> {
        // Not yet implemented — return None to fall through to legacy codegen
        None
    }

    /// Check if the IR pipeline is ready to fully replace the legacy path.
    ///
    /// Returns true only when all subsystems have been validated and the
    /// pipeline can produce correct output for the full test suite.
    pub fn is_ready_for_cutover(&self) -> bool {
        false
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
