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

use super::constraints::Constraint;
use super::effects::{self, EffectConstraint};
use super::node::parser_type_to_base_type;
use super::safety_type::{BaseType, EffectSet, OwnedType, Region, SafetyType};
use super::taint;
use super::{constraint_gen, EffectSolver, ExecutionValidator, IrFunction, Solver, TaintSolver};
use crate::analyzer::OwnershipMode;
use crate::parser::Type;
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

impl IrModule {
    /// Check resolved effects against declared capabilities from `wj.toml`.
    ///
    /// Returns a list of capability violation diagnostics. Each violation
    /// identifies the function and the undeclared effect.
    pub fn check_capabilities(
        &self,
        allowed: &EffectSet,
    ) -> Vec<IrDiagnostic> {
        let mut violations = Vec::new();
        for (fn_name, fn_effects) in &self.effect_sets {
            for effect in fn_effects.iter() {
                if !allowed.contains(effect) {
                    violations.push(IrDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: format!(
                            "Effect violation: function `{}` requires `{}` which is not declared in [app_capabilities]",
                            fn_name, effect,
                        ),
                        span: None,
                    });
                }
            }
        }
        violations
    }
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
        registry: &crate::analyzer::SignatureRegistry,
        struct_field_types: Option<&HashMap<String, HashMap<String, crate::parser::Type>>>,
    ) -> IrModule {
        let struct_field_types = struct_field_types.cloned().unwrap_or_default();
        let mut diagnostics = Vec::new();

        // Stage 1: Convert AnalyzedFunctions to IrFunctions (lossless bridge)
        let mut functions: Vec<IrFunction> = analyzed
            .iter()
            .map(|af| {
                let mut ir_fn = IrFunction::from_analyzed(af);
                ir_fn.body = super::lower::lower_body(&af.decl.body);
                ir_fn
            })
            .collect();

        diagnostics.push(IrDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: format!("IR: lowered {} functions from analyzer", functions.len()),
            span: None,
        });

        // Stage 2: Generate constraints and solve per-function
        let mut total_constraints = 0usize;
        let mut total_solver_diags = 0usize;
        let mut effect_constraints_forwarded = Vec::new();
        let mut taint_constraints_forwarded: Vec<taint::TaintConstraint> = Vec::new();
        let mut execution_constraints_forwarded: Vec<super::execution::ExecutionConstraint> =
            Vec::new();

        for (i, af) in analyzed.iter().enumerate() {
            let fc = constraint_gen::generate_constraints(af, Some(registry));
            total_constraints += fc.constraints.len();

            // Forward effect constraints to the domain solver.
            // HasEffects: direct effects from stdlib calls within this function.
            for c in fc.constraints.iter() {
                if let Constraint::HasEffects(_, ref eff_set) = c {
                    for eff in eff_set.iter() {
                        effect_constraints_forwarded.push(EffectConstraint::Performs {
                            function: fc.function_name.clone(),
                            effect: eff.clone(),
                        });
                    }
                }
            }
            // CallsInto: build call-graph edges from resolved callee names.
            for callee in &fc.call_targets {
                effect_constraints_forwarded.push(EffectConstraint::CallsInto {
                    caller: fc.function_name.clone(),
                    callee: callee.clone(),
                });
            }

            // Forward taint constraints from the AST walk.
            taint_constraints_forwarded.extend(fc.taint_constraints.iter().cloned());

            // Forward execution constraints from async/spawn call sites.
            execution_constraints_forwarded.extend(fc.execution_constraints.into_iter());

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

                // Ensure mutated parameters retain MutRef after solver write-back.
                // Returned params (e.g. `mut clip: T` consumed at return) stay owned.
                for param_name in &ir_fn.mutated_params.clone() {
                    if af.returned_parameters.contains(param_name) {
                        continue;
                    }
                    if let Some(st) = ir_fn.param_types.get_mut(param_name) {
                        if !matches!(st.ownership, OwnedType::MutRef(_)) {
                            st.ownership = OwnedType::MutRef(Region::fresh(0));
                        }
                    }
                }

                // Write local variable types from solver.
                for (local_name, &var) in &fc.var_map.locals {
                    let root_idx = var.0 as usize;
                    let base = result.types.get(root_idx).and_then(|t| t.as_ref());
                    let own = result.ownership.get(root_idx).and_then(|o| o.as_ref());
                    if base.is_some() || own.is_some() {
                        let entry = ir_fn.local_types.entry(local_name.clone()).or_insert_with(|| {
                            SafetyType::owned(BaseType::Inferred)
                        });
                        if let Some(b) = base {
                            if *b != BaseType::Inferred {
                                entry.base = b.clone();
                            }
                        }
                        if let Some(o) = own {
                            if *o != OwnedType::Inferred {
                                entry.ownership = o.clone();
                            }
                        }
                    }
                }
            }
        }

        apply_analyzer_readonly_string_params_to_ir(&mut functions, analyzed);
        clear_str_ref_for_owned_or_stored_params(&mut functions, analyzed);
        restore_owned_string_params_after_str_ref(&mut functions, analyzed);
        converge_impl_param_ownership(&mut functions, analyzed);
        finalize_ir_param_ownership_from_analyzer(&mut functions, analyzed);
        propagate_delegation_ownership(
            &mut functions,
            analyzed,
            registry,
            &struct_field_types,
        );

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

        // Stage 4: TaintSolver — load stdlib sources + sinks + forwarded constraints
        let taint_result = if self.config.enable_taint_tracking {
            let mut taint_solver = TaintSolver::new();
            for c in taint::stdlib_taint_sources() {
                taint_solver.add_constraint(c);
            }
            for c in taint::stdlib_sinks() {
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
            // Feed execution constraints from async/spawn call sites (WJ-CONC-01).
            for c in execution_constraints_forwarded {
                exec_validator.add_constraint(c);
            }
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
            let module = self.lower_to_ir(analyzed, registry, None);
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

/// Sync IR param ownership with converged analyzer body inference.
/// Solver/registry can lag for enum-payload stores, static-method passthrough borrows,
/// and field assignments — analyzer ownership is authoritative for formals.
fn finalize_ir_param_ownership_from_analyzer(
    functions: &mut [IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
) {
    let mut region_counter = 10_000u32;
    for (ir_fn, af) in functions.iter_mut().zip(analyzed.iter()) {
        for param in &af.decl.parameters {
            if param.name == "self" {
                continue;
            }
            let Some(mode) = af.inferred_ownership.get(&param.name) else {
                continue;
            };
            let Some(st) = ir_fn.param_types.get_mut(&param.name) else {
                continue;
            };
            match mode {
                OwnershipMode::Owned => {
                    st.ownership = OwnedType::Owned;
                    ir_fn.str_ref_params.remove(&param.name);
                    let is_plain_string = matches!(param.type_, Type::String)
                        || matches!(&param.type_, Type::Custom(name) if name == "string");
                    if is_plain_string {
                        st.base = BaseType::String;
                    }
                }
                OwnershipMode::Borrowed => {
                    if !af.mutated_parameters.contains(&param.name) {
                        let r = Region::fresh(region_counter);
                        region_counter += 1;
                        st.ownership = OwnedType::Ref(r);
                    }
                }
                OwnershipMode::MutBorrowed => {
                    let r = Region::fresh(region_counter);
                    region_counter += 1;
                    st.ownership = OwnedType::MutRef(r);
                }
            }
        }
    }
}

/// Re-apply analyzer/solver Owned semantics after readonly-string lowering.
fn restore_owned_string_params_after_str_ref(
    functions: &mut [IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
) {
    for (ir_fn, af) in functions.iter_mut().zip(analyzed.iter()) {
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            if param.name == "self" {
                continue;
            }
            let is_plain_string = matches!(param.type_, Type::String)
                || matches!(&param.type_, Type::Custom(name) if name == "string");
            if !is_plain_string {
                continue;
            }
            let force_owned = af.inferred_ownership.get(&param.name) == Some(&OwnershipMode::Owned)
                || af.returned_parameters.contains(&param.name);
            if !force_owned {
                continue;
            }
            if let Some(st) = ir_fn.param_types.get_mut(&param.name) {
                st.ownership = OwnedType::Owned;
                st.base = af
                    .inferred_param_types
                    .get(idx)
                    .map(parser_type_to_base_type)
                    .unwrap_or(BaseType::String);
            }
        }
    }
}

/// Drop str_ref markers for params the solver/analyzer kept owned (enum payload, struct store).
fn clear_str_ref_for_owned_or_stored_params(
    functions: &mut [IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
) {
    for (ir_fn, af) in functions.iter_mut().zip(analyzed.iter()) {
        ir_fn.str_ref_params.retain(|name| {
            if af.inferred_ownership.get(name) == Some(&OwnershipMode::Owned) {
                return false;
            }
            if let Some(st) = ir_fn.param_types.get(name) {
                if matches!(st.ownership, OwnedType::Owned) {
                    return false;
                }
            }
            true
        });
    }
}

/// Align IR param types with analyzer str_ref / readonly string inference so shadow
/// validation stays clean for `fn f(msg: string) { println("{}", msg) }`.
fn apply_analyzer_readonly_string_params_to_ir(
    functions: &mut [IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
) {
    for (ir_fn, af) in functions.iter_mut().zip(analyzed.iter()) {
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            if param.name == "self" {
                continue;
            }
            let is_plain_string = matches!(param.type_, Type::String)
                || matches!(&param.type_, Type::Custom(name) if name == "string");
            if !is_plain_string
                || matches!(
                    param.type_,
                    Type::Reference(_) | Type::MutableReference(_)
                )
            {
                continue;
            }
            let borrow = af.str_ref_optimizable_params.contains(&param.name)
                || matches!(
                    af.inferred_ownership.get(&param.name),
                    Some(OwnershipMode::Borrowed)
                );
            if !borrow {
                continue;
            }
            if af.inferred_ownership.get(&param.name) == Some(&OwnershipMode::Owned) {
                continue;
            }
            if let Some(st) = ir_fn.param_types.get_mut(&param.name) {
                // Solver-owned params (enum payload, struct field store) beat &str lowering.
                if matches!(st.ownership, OwnedType::Owned) {
                    continue;
                }
                st.ownership = OwnedType::Ref(Region::fresh(0));
                // Keep declared `String` base when the function returns a non-unit value so
                // shadow validation can surface IR String vs analyzer &str during migration
                // (e.g. `fn greet(name: string) -> i32 { name.len() }`). Unit-returning
                // pass-through helpers still sync base to `str` for clean shadow parity.
                let returns_non_unit = af
                    .decl
                    .return_type
                    .as_ref()
                    .is_some_and(|rt| parser_type_to_base_type(rt) != BaseType::Unit);
                if !returns_non_unit {
                    st.base = af
                        .inferred_param_types
                        .get(idx)
                        .map(parser_type_to_base_type)
                        .unwrap_or(BaseType::Custom("str".into()));
                }
            }
        }
    }
}

/// When any method in an impl uses an owned non-`self` parameter, sibling methods with
/// the same parameter index inherit owned semantics (TxnManager/MemoryEngine Key API).
fn converge_impl_param_ownership(
    functions: &mut [IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
) {
    use std::collections::{HashMap, HashSet};

    let mut impl_owned_indices: HashMap<String, HashSet<usize>> = HashMap::new();

    for (i, af) in analyzed.iter().enumerate() {
        let Some(parent) = af.decl.parent_type.as_ref() else {
            continue;
        };
        let ir_fn = &functions[i];
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            if param.name == "self" {
                continue;
            }
            if let Some(st) = ir_fn.param_types.get(&param.name) {
                if matches!(st.ownership, OwnedType::Owned)
                    && af.inferred_ownership.get(&param.name)
                        == Some(&OwnershipMode::Owned)
                {
                    impl_owned_indices
                        .entry(parent.clone())
                        .or_default()
                        .insert(idx);
                }
            }
        }
    }

    for (i, af) in analyzed.iter().enumerate() {
        let Some(parent) = af.decl.parent_type.as_ref() else {
            continue;
        };
        let Some(owned_indices) = impl_owned_indices.get(parent) else {
            continue;
        };
        let ir_fn = &mut functions[i];
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            if param.name == "self" {
                continue;
            }
            if owned_indices.contains(&idx) {
                if let Some(st) = ir_fn.param_types.get_mut(&param.name) {
                    st.ownership = OwnedType::Owned;
                }
            }
        }
    }
}

/// After impl convergence, upgrade wrapper methods that forward params to owned callees.
fn propagate_delegation_ownership(
    functions: &mut [IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
    registry: &crate::analyzer::SignatureRegistry,
    struct_field_types: &HashMap<String, HashMap<String, crate::parser::Type>>,
) {
    use crate::analyzer::OwnershipMode;

    for (caller_idx, af) in analyzed.iter().enumerate() {
        let Some((method, arg_names, receiver_type)) =
            extract_single_method_delegation(af.decl.body.as_slice(), &af.decl, struct_field_types)
        else {
            continue;
        };
        let caller_name = &functions[caller_idx].name;
        let arg_count = arg_names.len();
        let mut owned_args: Vec<String> = Vec::new();

        if let Some(callee_idx) = functions.iter().position(|f| {
            f.name.ends_with(&format!("::{method}")) && &f.name != caller_name
        }) {
            let callee_ir = &functions[callee_idx];
            let callee_params: Vec<_> = analyzed[callee_idx]
                .decl
                .parameters
                .iter()
                .filter(|p| p.name != "self")
                .map(|p| p.name.as_str())
                .collect();
            for (arg_idx, arg_name) in arg_names.iter().enumerate() {
                let Some(callee_param_name) = callee_params.get(arg_idx) else {
                    continue;
                };
                let Some(callee_st) = callee_ir.param_types.get(*callee_param_name) else {
                    continue;
                };
                if matches!(callee_st.ownership, OwnedType::Owned) {
                    owned_args.push(arg_name.clone());
                }
            }
        } else if let Some(receiver_type) = receiver_type.as_deref() {
            if let Some(sig) =
                registry.find_method_on_receiver_type(receiver_type, &method, arg_count)
            {
                collect_owned_delegation_args(sig, &arg_names, &mut owned_args);
            }
        } else if let Some(sig) =
            registry.find_delegation_callee(caller_name, &method, arg_count)
        {
            collect_owned_delegation_args(sig, &arg_names, &mut owned_args);
        }

        for arg_name in owned_args {
            if let Some(st) = functions[caller_idx].param_types.get_mut(&arg_name) {
                st.ownership = OwnedType::Owned;
            }
        }
    }
}

fn collect_owned_delegation_args(
    sig: &crate::analyzer::FunctionSignature,
    arg_names: &[String],
    owned_args: &mut Vec<String>,
) {
    use crate::analyzer::OwnershipMode;

    for (arg_idx, arg_name) in arg_names.iter().enumerate() {
        let param_idx = sig.arg_param_index(arg_idx);
        let owned = sig
            .param_ownership
            .get(param_idx)
            .is_some_and(|m| *m == OwnershipMode::Owned);
        if owned {
            owned_args.push(arg_name.clone());
        }
    }
}

fn extract_single_method_delegation<'ast>(
    body: &[&crate::parser::ast::core::Statement<'ast>],
    func: &crate::parser::ast::core::FunctionDecl<'ast>,
    struct_field_types: &HashMap<String, HashMap<String, crate::parser::Type>>,
) -> Option<(String, Vec<String>, Option<String>)> {
    use crate::parser::ast::core::{Expression, Statement};
    let expr = single_statement_expression(body)?;
    let Expression::MethodCall {
        method,
        arguments,
        object,
        ..
    } = expr
    else {
        return None;
    };
    let mut arg_names = Vec::new();
    for (_label, arg) in arguments {
        if let Expression::Identifier { name, .. } = arg {
            if name != "self" {
                arg_names.push(name.clone());
            }
        } else {
            return None;
        }
    }
    let receiver_type = infer_delegation_receiver_type(object, func, struct_field_types);
    Some((method.clone(), arg_names, receiver_type))
}

fn infer_delegation_receiver_type<'ast>(
    object: &crate::parser::ast::core::Expression<'ast>,
    func: &crate::parser::ast::core::FunctionDecl<'ast>,
    struct_field_types: &HashMap<String, HashMap<String, crate::parser::Type>>,
) -> Option<String> {
    use crate::parser::ast::core::Expression;
    use crate::parser::Type;

    fn strip_generics(name: &str) -> String {
        name.split('<').next().unwrap_or(name).to_string()
    }

    fn type_to_struct_base(ty: &Type) -> Option<String> {
        match ty {
            Type::Custom(name) => Some(strip_generics(name)),
            Type::Parameterized(base, _) => Some(strip_generics(base)),
            Type::Reference(inner) | Type::MutableReference(inner) => type_to_struct_base(inner),
            _ => None,
        }
    }

    match object {
        Expression::Identifier { name, .. } if name == "self" => func
            .parent_type
            .as_ref()
            .map(|p| strip_generics(p)),
        Expression::Identifier { name, .. } => func
            .parameters
            .iter()
            .find(|p| &p.name == name)
            .and_then(|p| type_to_struct_base(&p.type_)),
        Expression::FieldAccess {
            object: inner,
            field,
            ..
        } => {
            let inner_base = infer_delegation_receiver_type(inner, func, struct_field_types)?;
            crate::type_inference::struct_field_registry::lookup_struct_field_map(
                struct_field_types,
                &inner_base,
                &HashMap::new(),
                &HashMap::new(),
            )
            .and_then(|fields| fields.get(field.as_str()))
            .and_then(type_to_struct_base)
        }
        _ => None,
    }
}

fn single_statement_expression<'a>(
    body: &[&'a crate::parser::ast::core::Statement<'a>],
) -> Option<&'a crate::parser::ast::core::Expression<'a>> {
    use crate::parser::ast::core::{Expression, Statement};
    match body.len() {
        0 => None,
        1 => match body[0] {
            Statement::Return { value: Some(e), .. } => Some(e),
            Statement::Expression { expr, .. } => Some(expr),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_source(
        source: &str,
    ) -> (
        Vec<crate::analyzer::AnalyzedFunction<'static>>,
        crate::analyzer::SignatureRegistry,
    ) {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        // Leak the parser so its arena allocators keep AST nodes alive
        // (required because constraint_gen now walks the AST body)
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        (analyzed, registry)
    }

    #[test]
    fn test_pipeline_runs_all_domain_solvers() {
        let (analyzed, registry) = analyze_source("pub fn hello() -> i32 { 42 }");
        let mut pipeline = IrPipeline::new();
        let module = pipeline.lower_to_ir(&analyzed, &registry, None);

        assert_eq!(module.functions.len(), 1);
        assert!(module.taint_errors.is_empty());
        assert!(module.execution_errors.is_empty());
        assert!(module.execution_warnings.is_empty());
        // Stdlib effect declarations should be loaded
        assert!(
            !module.effect_sets.is_empty(),
            "stdlib effects should be populated"
        );
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
        let module = pipeline.lower_to_ir(&analyzed, &registry, None);

        assert_eq!(module.functions.len(), 1);
        assert!(module.effect_sets.is_empty());
        assert!(module.taint_errors.is_empty());
        assert!(module.execution_errors.is_empty());
    }

    #[test]
    fn test_pipeline_diagnostics_include_solver_summary() {
        let (analyzed, registry) = analyze_source("pub fn add(x: i32, y: i32) -> i32 { x }");
        let mut pipeline = IrPipeline::new();
        let module = pipeline.lower_to_ir(&analyzed, &registry, None);

        let has_solver_summary = module
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Solver:"));
        assert!(has_solver_summary, "should have solver summary diagnostic");

        let has_ir_summary = module
            .diagnostics
            .iter()
            .any(|d| d.message.contains("IR summary:"));
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
        let module = pipeline.lower_to_ir(&analyzed, &registry, None);

        assert_eq!(module.functions.len(), 2);
        assert!(module.taint_errors.is_empty());
    }

    #[test]
    fn test_check_capabilities_passes_when_allowed() {
        use crate::ir::safety_type::{Effect, EffectSet};

        let mut effect_sets = HashMap::new();
        let mut fs_effects = EffectSet::pure();
        fs_effects.insert(Effect::FsRead);
        effect_sets.insert("read_config".to_string(), fs_effects);

        let module = IrModule {
            functions: vec![],
            diagnostics: vec![],
            effect_sets,
            taint_errors: vec![],
            execution_errors: vec![],
            execution_warnings: vec![],
        };

        let mut allowed = EffectSet::pure();
        allowed.insert(Effect::FsRead);
        allowed.insert(Effect::FsWrite);

        let violations = module.check_capabilities(&allowed);
        assert!(violations.is_empty(), "should pass with sufficient capabilities");
    }

    #[test]
    fn test_check_capabilities_fails_when_undeclared() {
        use crate::ir::safety_type::{Effect, EffectSet};

        let mut effect_sets = HashMap::new();
        let mut fn_effects = EffectSet::pure();
        fn_effects.insert(Effect::NetEgress);
        effect_sets.insert("fetch_data".to_string(), fn_effects);

        let module = IrModule {
            functions: vec![],
            diagnostics: vec![],
            effect_sets,
            taint_errors: vec![],
            execution_errors: vec![],
            execution_warnings: vec![],
        };

        let allowed = EffectSet::single(Effect::FsRead);

        let violations = module.check_capabilities(&allowed);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("fetch_data"));
        assert!(violations[0].message.contains("net_egress"));
    }
}
