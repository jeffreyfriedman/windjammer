//! Unified Numeric Inference — the single entry point for all numeric type inference.
//!
//! Wraps FloatInference and IntInference collectors as private implementation details
//! behind a clean public API. Both collectors walk the AST independently, then solve
//! their constraints independently (float and int are separate type domains).
//!
//! Architecture:
//!   AST → FloatCollector.collect() → float constraints → float solve → F32/F64
//!   AST → IntCollector.collect()   → int constraints   → int solve   → I32/U64/etc.
//!
//! The unified engine:
//! - Provides a single `infer_program` entry point
//! - Supports parallel per-file collection + merge for library builds
//! - Exposes `get_float_type` / `get_int_type` for codegen consumption
//! - Hides the dual-collector implementation from all consumers

use crate::parser::ast::core::Expression;
use crate::parser::ast::types::Type;
use crate::parser::Program;
use crate::type_inference::float_inference::{FloatInference, FloatType};
use crate::type_inference::int_inference::{IntInference, IntType};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unified numeric inference engine.
///
/// This is the sole public interface for numeric type inference in the compiler.
/// All pipeline code should create, configure, and consume this type rather than
/// using FloatInference or IntInference directly.
pub struct UnifiedNumericInference {
    float_collector: FloatInference,
    int_collector: IntInference,
    pub errors: Vec<String>,
    solved: bool,
}

impl Clone for UnifiedNumericInference {
    fn clone(&self) -> Self {
        Self {
            float_collector: self.float_collector.clone(),
            int_collector: self.int_collector.clone(),
            errors: self.errors.clone(),
            solved: self.solved,
        }
    }
}

impl Default for UnifiedNumericInference {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedNumericInference {
    pub fn new() -> Self {
        Self {
            float_collector: FloatInference::new(),
            int_collector: IntInference::new(),
            errors: Vec::new(),
            solved: false,
        }
    }

    /// Construct from an already-solved FloatInference (test compatibility).
    pub fn from_float_only(float_inf: FloatInference) -> Self {
        Self {
            float_collector: float_inf,
            int_collector: IntInference::new(),
            errors: Vec::new(),
            solved: true,
        }
    }

    /// Construct from an already-solved IntInference (test compatibility).
    pub fn from_int_only(int_inf: IntInference) -> Self {
        Self {
            float_collector: FloatInference::new(),
            int_collector: int_inf,
            errors: Vec::new(),
            solved: true,
        }
    }

    // --- Configuration setters (delegated to both collectors) ---

    pub fn set_current_file(&mut self, file: &str) -> usize {
        let fid = self.float_collector.set_current_file(file.to_string());
        self.int_collector.set_current_file(file.to_string());
        fid
    }

    pub fn set_debug_source(&mut self, source: &str) {
        self.float_collector.set_debug_source(source);
    }

    pub fn set_source_root(&mut self, root: &std::path::Path) {
        self.float_collector.set_source_root(root);
    }

    pub fn set_global_function_signatures(
        &mut self,
        sigs: &HashMap<String, (Vec<Type>, Option<Type>)>,
    ) {
        self.float_collector
            .set_global_function_signatures(sigs.clone());
        self.int_collector
            .set_global_function_signatures(sigs.clone());
    }

    pub fn set_global_struct_field_types(
        &mut self,
        fields: &HashMap<String, HashMap<String, Type>>,
    ) {
        self.float_collector.set_global_struct_field_types(fields);
        self.int_collector.set_global_struct_field_types(fields);
    }

    pub fn set_current_file_module_path(&mut self, path: Vec<String>) {
        self.float_collector
            .set_current_file_module_path(path.clone());
        self.int_collector.set_current_file_module_path(path);
    }

    pub fn set_struct_defining_module_paths(&mut self, paths: HashMap<String, Vec<Vec<String>>>) {
        self.float_collector
            .set_struct_defining_module_paths(paths.clone());
        self.int_collector.set_struct_defining_module_paths(paths);
    }

    pub fn set_module_re_exports(&mut self, re_exports: HashMap<String, HashMap<String, String>>) {
        self.float_collector
            .set_module_re_exports(re_exports.clone());
        self.int_collector.set_module_re_exports(re_exports);
    }

    pub fn set_external_crate_metadata_paths(&mut self, paths: &HashMap<String, PathBuf>) {
        self.float_collector
            .set_external_crate_metadata_paths(paths);
        // Int domain must also see cross-crate formals (e.g. `double(x: i32)` via
        // `--use-build-dir` metadata) so literals emit `_i32` not default `_i64`.
        self.int_collector
            .set_external_crate_metadata_paths(paths);
    }

    pub fn reset_imported_type_registry(&mut self) {
        self.float_collector.reset_imported_type_registry();
        self.int_collector.reset_imported_type_registry();
    }

    // --- Core inference pipeline ---

    /// Register types, signatures, and imports from the program.
    pub fn prepare_program<'ast>(&mut self, program: &Program<'ast>) {
        self.float_collector.prepare_program(program);
        self.int_collector.prepare_program(program);
    }

    /// Walk the AST and collect constraints (both float and int).
    pub fn collect_program_constraints<'ast>(&mut self, program: &Program<'ast>) {
        self.float_collector.collect_program_constraints(program);
        self.int_collector.collect_program_constraints(program);
    }

    /// Solve: run each domain's solver independently.
    ///
    /// Float and int constraints are solved separately because the same
    /// expression can legitimately be typed in both domains (e.g. `count: i32`
    /// is I32 in the int domain and may appear in a float context like
    /// `speed + count as f32`). Merging them into one solver causes false
    /// conflicts.
    pub fn finish_solve(&mut self) {
        self.float_collector.finish_solve();
        self.int_collector.finish_solve();

        for err in &self.float_collector.errors {
            self.errors.push(err.clone());
        }
        for err in &self.int_collector.errors {
            self.errors.push(err.clone());
        }

        self.solved = true;
    }

    /// Single-file convenience: prepare + collect + solve.
    pub fn infer_program<'ast>(&mut self, program: &Program<'ast>) {
        self.reset_imported_type_registry();
        self.prepare_program(program);
        self.collect_program_constraints(program);
        self.finish_solve();
    }

    /// Merge parallel collection results (for library multipass).
    pub fn merge_parallel_state(&mut self, other: Self) {
        self.float_collector
            .merge_parallel_state(other.float_collector);
        self.int_collector.merge_parallel_state(other.int_collector);
    }

    // --- Codegen-facing lookup ---

    /// Look up the inferred float type for an expression.
    pub fn get_float_type(&self, expr: &Expression) -> FloatType {
        self.float_collector.get_float_type(expr)
    }

    /// Look up the inferred integer type for an expression.
    pub fn get_int_type(&self, expr: &Expression) -> IntType {
        self.int_collector.get_int_type(expr)
    }

    /// Export inferred variable types for IDE/MCP consumers.
    pub fn export_var_types(&self) -> HashMap<String, String> {
        self.int_collector.export_var_types()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_inference_basic() {
        let unified = UnifiedNumericInference::new();
        assert!(unified.errors.is_empty());
        assert!(!unified.solved);
    }

    #[test]
    fn test_infer_program_single_file() {
        let source = "pub fn add(a: f32, b: f32) -> f32 {\n    a + b\n}\n";
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(
            tokens,
            "test.wj".to_string(),
            source.to_string(),
        );
        let program = parser.parse().expect("parse");

        let mut unified = UnifiedNumericInference::new();
        unified.infer_program(&program);

        assert!(
            unified.errors.is_empty(),
            "no errors for valid program: {:?}",
            unified.errors
        );
    }
}
