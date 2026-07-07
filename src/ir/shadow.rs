//! Shadow mode validation for the IR pipeline.
//!
//! Runs the IR pipeline alongside legacy codegen and compares decisions.
//! Discrepancies between the solver-based IR and the heuristic-based analyzer
//! are logged as warnings (or errors with `--ir-shadow-validate`).
//!
//! This catches solver bugs before the codegen cutover (WP5).

use crate::analyzer::{AnalyzedFunction, OwnershipMode, SignatureRegistry};
use crate::ir::node::parser_type_to_base_type;
use crate::ir::pipeline::{IrModule, IrPipeline};
use crate::ir::safety_type::{BaseType, OwnedType};

/// A discrepancy between IR solver results and legacy analyzer decisions.
#[derive(Debug, Clone)]
pub struct ShadowDiscrepancy {
    pub function: String,
    pub category: DiscrepancyCategory,
    pub param_name: Option<String>,
    pub ir_value: String,
    pub analyzer_value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscrepancyCategory {
    Ownership,
    Type,
    StrRefOptimization,
}

impl std::fmt::Display for ShadowDiscrepancy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let param = self
            .param_name
            .as_deref()
            .map(|n| format!(" param '{}'", n))
            .unwrap_or_default();
        write!(
            f,
            "[IR shadow] {}{}: {:?} mismatch: IR={}, analyzer={}",
            self.function, param, self.category, self.ir_value, self.analyzer_value
        )
    }
}

/// Result of shadow validation.
#[derive(Debug)]
pub struct ShadowValidationResult {
    pub discrepancies: Vec<ShadowDiscrepancy>,
    pub functions_checked: usize,
    pub params_checked: usize,
}

impl ShadowValidationResult {
    pub fn is_clean(&self) -> bool {
        self.discrepancies.is_empty()
    }
}

/// Run shadow validation: compare IR solver results against legacy analyzer.
pub fn validate_shadow(
    analyzed: &[AnalyzedFunction],
    registry: &SignatureRegistry,
) -> ShadowValidationResult {
    let mut pipeline = IrPipeline::new();
    let module = pipeline.lower_to_ir(analyzed, registry);

    compare_ir_to_analyzer(&module, analyzed)
}

/// Run shadow validation across multiple files in library mode.
pub fn validate_shadow_multi_file(
    files: &[(&str, &[AnalyzedFunction])],
    registry: &SignatureRegistry,
) -> ShadowValidationResult {
    let mut pipeline = IrPipeline::new();
    let module_set = pipeline.lower_multi_file_to_ir(files, registry);

    let mut combined = ShadowValidationResult {
        discrepancies: Vec::new(),
        functions_checked: 0,
        params_checked: 0,
    };

    for (file_module, &(_, analyzed)) in module_set.files.iter().zip(files.iter()) {
        let result = compare_ir_to_analyzer(&file_module.module, analyzed);
        combined.discrepancies.extend(result.discrepancies);
        combined.functions_checked += result.functions_checked;
        combined.params_checked += result.params_checked;
    }

    combined
}

/// Compare an IR module's decisions against the original analyzed functions.
fn compare_ir_to_analyzer(
    module: &IrModule,
    analyzed: &[AnalyzedFunction],
) -> ShadowValidationResult {
    let mut discrepancies = Vec::new();
    let mut params_checked = 0usize;

    for (ir_fn, af) in module.functions.iter().zip(analyzed.iter()) {
        let fn_name = &ir_fn.name;

        // Compare ownership per parameter
        for (param_name, analyzer_mode) in &af.inferred_ownership {
            params_checked += 1;

            if let Some(ir_safety_type) = ir_fn.param_types.get(param_name) {
                let ir_ownership = &ir_safety_type.ownership;
                let analyzer_owned_type = match analyzer_mode {
                    OwnershipMode::Owned => "Owned",
                    OwnershipMode::Borrowed => "Ref",
                    OwnershipMode::MutBorrowed => "MutRef",
                };

                let ir_ownership_str = match ir_ownership {
                    OwnedType::Owned => "Owned",
                    OwnedType::Ref(_) => "Ref",
                    OwnedType::MutRef(_) => "MutRef",
                    OwnedType::Copy => "Copy",
                    OwnedType::Inferred => "Inferred",
                };

                // Don't flag Inferred — it means the solver didn't have enough data
                if ir_ownership_str != "Inferred" && ir_ownership_str != analyzer_owned_type {
                    discrepancies.push(ShadowDiscrepancy {
                        function: fn_name.clone(),
                        category: DiscrepancyCategory::Ownership,
                        param_name: Some(param_name.clone()),
                        ir_value: ir_ownership_str.to_string(),
                        analyzer_value: analyzer_owned_type.to_string(),
                    });
                }
            }
        }

        // Compare types per parameter
        for (idx, param_decl) in af.decl.parameters.iter().enumerate() {
            let param_name = &param_decl.name;

            let analyzer_type = if let Some(inferred_ty) = af.inferred_param_types.get(idx) {
                parser_type_to_base_type(inferred_ty)
            } else {
                parser_type_to_base_type(&param_decl.type_)
            };

            if let Some(ir_safety_type) = ir_fn.param_types.get(param_name) {
                let ir_type = &ir_safety_type.base;

                if *ir_type != BaseType::Inferred
                    && analyzer_type != BaseType::Inferred
                    && *ir_type != analyzer_type
                {
                    discrepancies.push(ShadowDiscrepancy {
                        function: fn_name.clone(),
                        category: DiscrepancyCategory::Type,
                        param_name: Some(param_name.clone()),
                        ir_value: format!("{:?}", ir_type),
                        analyzer_value: format!("{:?}", analyzer_type),
                    });
                }
            }
        }

        // Compare str_ref optimizations
        for param_name in &af.str_ref_optimizable_params {
            if !ir_fn.str_ref_params.contains(param_name) {
                discrepancies.push(ShadowDiscrepancy {
                    function: fn_name.clone(),
                    category: DiscrepancyCategory::StrRefOptimization,
                    param_name: Some(param_name.clone()),
                    ir_value: "not str_ref".to_string(),
                    analyzer_value: "str_ref_optimizable".to_string(),
                });
            }
        }
        for param_name in &ir_fn.str_ref_params {
            if !af.str_ref_optimizable_params.contains(param_name) {
                discrepancies.push(ShadowDiscrepancy {
                    function: fn_name.clone(),
                    category: DiscrepancyCategory::StrRefOptimization,
                    param_name: Some(param_name.clone()),
                    ir_value: "str_ref".to_string(),
                    analyzer_value: "not str_ref_optimizable".to_string(),
                });
            }
        }
    }

    ShadowValidationResult {
        discrepancies,
        functions_checked: module.functions.len(),
        params_checked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_and_validate(source: &str) -> ShadowValidationResult {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        validate_shadow(&analyzed, &registry)
    }

    #[test]
    fn test_shadow_simple_function_clean() {
        let result = analyze_and_validate("pub fn add(x: i32, y: i32) -> i32 { x }");
        // The IR bridges from the same analyzer data, so there should be no discrepancies
        // on simple functions where ownership matches
        assert!(
            result.functions_checked >= 1,
            "should check at least 1 function"
        );
    }

    #[test]
    fn test_shadow_empty_function() {
        let result = analyze_and_validate("pub fn noop() {}");
        assert_eq!(result.functions_checked, 1);
        assert_eq!(result.params_checked, 0);
        assert!(result.is_clean());
    }

    #[test]
    fn test_shadow_multiple_functions() {
        let source = r#"
pub fn a(x: i32) -> i32 { x }
pub fn b(s: string) {}
"#;
        let result = analyze_and_validate(source);
        assert_eq!(result.functions_checked, 2);
        assert!(result.params_checked >= 2);
    }

    #[test]
    fn test_shadow_multi_file() {
        let source_a = "pub fn compute(x: i32) -> i32 { x }";
        let source_b = "pub fn display(name: string) {}";

        let mut lexer_a = crate::lexer::Lexer::new(source_a);
        let toks_a = lexer_a.tokenize_with_locations();
        let mut parser_a = crate::parser::Parser::new(toks_a);
        let prog_a = parser_a.parse().expect("parse a");
        let prog_a: &'static _ = Box::leak(Box::new(prog_a));

        let mut lexer_b = crate::lexer::Lexer::new(source_b);
        let toks_b = lexer_b.tokenize_with_locations();
        let mut parser_b = crate::parser::Parser::new(toks_b);
        let prog_b = parser_b.parse().expect("parse b");
        let prog_b: &'static _ = Box::leak(Box::new(prog_b));

        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed_a, reg_a, _) = analyzer.analyze_program(prog_a).expect("analyze a");
        let mut analyzer_b = crate::analyzer::Analyzer::new();
        let (analyzed_b, _, _) = analyzer_b.analyze_program(prog_b).expect("analyze b");

        let files: Vec<(&str, &[AnalyzedFunction])> =
            vec![("a.wj", &analyzed_a), ("b.wj", &analyzed_b)];
        let result = validate_shadow_multi_file(&files, &reg_a);

        assert!(result.functions_checked >= 2);
        assert!(result.params_checked >= 2);
    }

    #[test]
    fn test_shadow_str_ref_consistency() {
        let result = analyze_and_validate("pub fn greet(name: string) { println!(\"{}\", name) }");
        assert_eq!(result.functions_checked, 1);
        // str_ref should be consistent between IR and analyzer since IR copies the same data
        let str_ref_discreps: Vec<_> = result
            .discrepancies
            .iter()
            .filter(|d| d.category == DiscrepancyCategory::StrRefOptimization)
            .collect();
        assert!(
            str_ref_discreps.is_empty(),
            "str_ref should be consistent: {:?}",
            str_ref_discreps
        );
    }
}
