//! Constraint generation from AnalyzedFunction.
//!
//! Walks the analyzer output and emits constraints for the unified solver.
//! This is Phase 2 of the IR pipeline: AST → Constraints → Solver → SafetyType.
//!
//! The generator creates a `ConstraintVar` for each parameter, return value,
//! and relevant local binding, then emits:
//!   - `TypeIs` for declared/inferred types
//!   - `TypeEquals` for assignments and return type unification
//!   - `OwnershipIs` for analyzer-inferred ownership modes
//!   - `NeedsClone` for auto-clone insertion points
//!   - `IsNumeric/IsInteger/IsFloat` for numeric constraint domains

use crate::analyzer::{AnalyzedFunction, OwnershipMode};
use crate::ir::constraints::{Constraint, ConstraintSet, ConstraintVar};
use crate::ir::node::parser_type_to_base_type;
use crate::ir::safety_type::{BaseType, OwnedType, Region};
use std::collections::HashMap;

/// Maps between function entities and their constraint variables.
#[derive(Debug, Clone)]
pub struct ConstraintVarMap {
    /// Parameter name → constraint var
    pub params: HashMap<String, ConstraintVar>,
    /// Return value constraint var
    pub return_var: ConstraintVar,
    /// Local variable name → constraint var (populated during body walk)
    pub locals: HashMap<String, ConstraintVar>,
}

/// Result of constraint generation for a single function.
#[derive(Debug)]
pub struct FunctionConstraints {
    pub function_name: String,
    pub constraints: ConstraintSet,
    pub var_map: ConstraintVarMap,
}

/// Generate constraints from an analyzed function.
///
/// This is a single-pass walk over the function's metadata that converts
/// analyzer-produced annotations into typed constraints for the solver.
pub fn generate_constraints(analyzed: &AnalyzedFunction<'_>) -> FunctionConstraints {
    let mut cs = ConstraintSet::new();
    let mut param_vars = HashMap::new();
    let mut region_counter: u32 = 1;

    // 1. Create constraint vars for each parameter and emit type + ownership constraints
    for (param_name, ownership_mode) in &analyzed.inferred_ownership {
        let var = cs.fresh_var();
        param_vars.insert(param_name.clone(), var);

        // Emit ownership constraint from analyzer
        let ownership = match ownership_mode {
            OwnershipMode::Owned => OwnedType::Owned,
            OwnershipMode::Borrowed => {
                let r = Region::fresh(region_counter);
                region_counter += 1;
                OwnedType::Ref(r)
            }
            OwnershipMode::MutBorrowed => {
                let r = Region::fresh(region_counter);
                region_counter += 1;
                OwnedType::MutRef(r)
            }
        };
        cs.add(Constraint::OwnershipIs(var, ownership));

        // Emit type constraint from declared parameter type
        let param_idx = analyzed
            .decl
            .parameters
            .iter()
            .position(|p| p.name == *param_name);
        if let Some(idx) = param_idx {
            let declared_type = &analyzed.decl.parameters[idx].type_;
            let base = parser_type_to_base_type(declared_type);
            if base != BaseType::Inferred {
                cs.add(Constraint::TypeIs(var, base.clone()));
                emit_numeric_class_constraint(&mut cs, var, &base);
            }
        }

        // Override with inferred param type if available
        if let Some(idx) = param_idx {
            if let Some(inferred_ty) = analyzed.inferred_param_types.get(idx) {
                let base = parser_type_to_base_type(inferred_ty);
                if base != BaseType::Inferred {
                    cs.add(Constraint::TypeIs(var, base.clone()));
                    emit_numeric_class_constraint(&mut cs, var, &base);
                }
            }
        }

        // str_ref optimization indicates String → &str coercion
        if analyzed.str_ref_optimizable_params.contains(param_name) {
            let str_var = cs.fresh_var();
            cs.add(Constraint::TypeIs(str_var, BaseType::String));
            cs.add(Constraint::OwnershipIs(
                str_var,
                OwnedType::Ref(Region::fresh(region_counter)),
            ));
            region_counter += 1;
        }
    }

    // 2. Create return type constraint var
    let return_var = cs.fresh_var();
    let return_base = analyzed
        .decl
        .return_type
        .as_ref()
        .map(|rt| parser_type_to_base_type(rt))
        .unwrap_or(BaseType::Unit);
    if return_base != BaseType::Inferred {
        cs.add(Constraint::TypeIs(return_var, return_base.clone()));
        emit_numeric_class_constraint(&mut cs, return_var, &return_base);
    }
    cs.add(Constraint::OwnershipIs(return_var, OwnedType::Owned));

    // 3. Emit NeedsClone constraints from auto-clone analysis
    let mut local_vars = HashMap::new();
    for ((var_name, _stmt_idx), _reason) in &analyzed.auto_clone_analysis.clone_sites {
        let var = if let Some(v) = param_vars.get(var_name) {
            *v
        } else if let Some(v) = local_vars.get(var_name) {
            *v
        } else {
            let v = cs.fresh_var();
            local_vars.insert(var_name.clone(), v);
            v
        };
        cs.add(Constraint::NeedsClone(var));
    }

    // 4. Emit constraints for mutated variables
    for var_name in &analyzed.mutated_variables {
        if !local_vars.contains_key(var_name) && !param_vars.contains_key(var_name) {
            let v = cs.fresh_var();
            local_vars.insert(var_name.clone(), v);
        }
    }
    for param_name in &analyzed.mutated_parameters {
        if let Some(&var) = param_vars.get(param_name) {
            let r = Region::fresh(region_counter);
            region_counter += 1;
            cs.add(Constraint::OwnershipIs(var, OwnedType::MutRef(r)));
        }
    }

    let _ = region_counter; // suppress warning

    FunctionConstraints {
        function_name: analyzed.decl.name.to_string(),
        constraints: cs,
        var_map: ConstraintVarMap {
            params: param_vars,
            return_var,
            locals: local_vars,
        },
    }
}

/// Emit numeric class constraints (IsNumeric/IsInteger/IsFloat) based on base type.
fn emit_numeric_class_constraint(cs: &mut ConstraintSet, var: ConstraintVar, base: &BaseType) {
    match base {
        BaseType::I8
        | BaseType::I16
        | BaseType::I32
        | BaseType::I64
        | BaseType::I128
        | BaseType::U8
        | BaseType::U16
        | BaseType::U32
        | BaseType::U64
        | BaseType::U128 => {
            cs.add(Constraint::IsInteger(var));
            cs.add(Constraint::IsNumeric(var));
        }
        BaseType::F32 | BaseType::F64 => {
            cs.add(Constraint::IsFloat(var));
            cs.add(Constraint::IsNumeric(var));
        }
        _ => {}
    }
}

/// Generate constraints for a batch of analyzed functions.
pub fn generate_module_constraints(analyzed: &[AnalyzedFunction<'_>]) -> Vec<FunctionConstraints> {
    analyzed.iter().map(generate_constraints).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::constraints::Constraint;

    fn count_constraints(fc: &FunctionConstraints, pred: impl Fn(&Constraint) -> bool) -> usize {
        fc.constraints.iter().filter(|c| pred(c)).count()
    }

    #[test]
    fn test_empty_function_produces_return_var() {
        let source = "pub fn empty() {}";
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

        assert!(!analyzed.is_empty());
        let fc = generate_constraints(&analyzed[0]);

        assert_eq!(fc.function_name, "empty");
        assert!(fc.var_map.params.is_empty());
        // Return var should have TypeIs(Unit) and OwnershipIs(Owned)
        let has_unit =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::Unit)));
        assert!(has_unit >= 1, "should have TypeIs(Unit) for return");
    }

    #[test]
    fn test_typed_params_emit_type_constraints() {
        let source = "pub fn add(x: i32, y: f64) -> i32 { x }";
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

        let fc = generate_constraints(&analyzed[0]);

        assert_eq!(fc.var_map.params.len(), 2);

        let has_i32 = count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::I32)));
        let has_f64 = count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::F64)));
        assert!(has_i32 >= 1, "should have TypeIs(I32) for x and return");
        assert!(has_f64 >= 1, "should have TypeIs(F64) for y");

        let has_is_integer = count_constraints(&fc, |c| matches!(c, Constraint::IsInteger(_)));
        let has_is_float = count_constraints(&fc, |c| matches!(c, Constraint::IsFloat(_)));
        assert!(has_is_integer >= 1, "should have IsInteger constraints");
        assert!(has_is_float >= 1, "should have IsFloat constraints");
    }

    #[test]
    fn test_ownership_constraints_emitted() {
        let source = r#"
struct Point { x: f64, y: f64 }
pub fn mutate(p: Point) {
    p.x = 1.0
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

        let mutate_fn = analyzed.iter().find(|f| f.decl.name == "mutate").unwrap();
        let fc = generate_constraints(mutate_fn);

        let has_ownership = count_constraints(&fc, |c| matches!(c, Constraint::OwnershipIs(..)));
        assert!(
            has_ownership >= 1,
            "should have OwnershipIs constraints for params + return"
        );
    }

    #[test]
    fn test_module_constraints_batch() {
        let source = r#"
pub fn a() -> i32 { 1 }
pub fn b(x: string) {}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

        let all = generate_module_constraints(&analyzed);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].function_name, "a");
        assert_eq!(all[1].function_name, "b");
    }
}
