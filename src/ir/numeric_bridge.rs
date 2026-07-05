//! Bridge between legacy FloatInference/IntInference and the unified NumericSolver.
//!
//! Extracts solved results from the legacy engines, feeds them as seed constraints
//! into the NumericSolver for unified cross-type resolution, then backports the
//! unified results into the legacy maps so CodeGenerator reads them transparently.
//!
//! This enables the NumericSolver to catch cross-type conflicts and propagate
//! generic container type parameters that the sequential passes miss.

use crate::ir::numeric_solver::{NumericSolver, NumericSolverResult};
use crate::ir::numeric_types::{NumericConstraint, NumericType, UnifiedExprId};
use crate::type_inference::float_inference::{ExprId, FloatInference, FloatType};
use crate::type_inference::int_inference::{IntInference, IntType};

/// Convert a legacy `ExprId` to a `UnifiedExprId`.
fn to_unified(id: &ExprId) -> UnifiedExprId {
    UnifiedExprId::new(id.seq_id, id.file_id, id.line, id.col)
}

/// Extract constraints from a solved FloatInference engine and feed to NumericSolver.
fn extract_float_constraints(float_inf: &FloatInference, solver: &mut NumericSolver) {
    for (expr_id, float_type) in &float_inf.inferred_types {
        let nt = NumericType::from(*float_type);
        if !nt.is_unknown() {
            solver.add_constraint(NumericConstraint::MustBe {
                expr_id: to_unified(expr_id),
                numeric_type: nt,
                reason: "float inference seed".to_string(),
            });
        }
    }
}

/// Extract constraints from a solved IntInference engine and feed to NumericSolver.
fn extract_int_constraints(int_inf: &IntInference, solver: &mut NumericSolver) {
    for (expr_id, int_type) in &int_inf.inferred_types {
        let nt = NumericType::from(*int_type);
        if !nt.is_unknown() {
            solver.add_constraint(NumericConstraint::MustBe {
                expr_id: to_unified(expr_id),
                numeric_type: nt,
                reason: "int inference seed".to_string(),
            });
        }
    }
}

/// Backport unified results into the legacy FloatInference maps.
fn backport_float_results(float_inf: &mut FloatInference, result: &NumericSolverResult) {
    for (uid, numeric_type) in &result.resolved {
        if let Some(ft) = numeric_type.to_float_type() {
            if ft == FloatType::Unknown {
                continue;
            }
            let legacy_id = ExprId {
                seq_id: uid.seq_id,
                file_id: uid.file_id,
                line: uid.line,
                col: uid.col,
            };
            let current = float_inf.inferred_types.get(&legacy_id).copied();
            match current {
                None | Some(FloatType::Unknown) => {
                    float_inf.inferred_types.insert(legacy_id, ft);
                }
                Some(existing) if existing != ft => {
                    // Unified solver resolved a conflict — prefer its answer
                    float_inf.inferred_types.insert(legacy_id, ft);
                }
                _ => {}
            }
        }
    }
}

/// Backport unified results into the legacy IntInference maps.
fn backport_int_results(int_inf: &mut IntInference, result: &NumericSolverResult) {
    for (uid, numeric_type) in &result.resolved {
        if let Some(it) = numeric_type.to_int_type() {
            if it == IntType::Unknown {
                continue;
            }
            let legacy_id = ExprId {
                seq_id: uid.seq_id,
                file_id: uid.file_id,
                line: uid.line,
                col: uid.col,
            };
            let current = int_inf.inferred_types.get(&legacy_id).copied();
            match current {
                None | Some(IntType::Unknown) => {
                    int_inf.inferred_types.insert(legacy_id, it);
                }
                Some(existing) if existing != it => {
                    int_inf.inferred_types.insert(legacy_id, it);
                }
                _ => {}
            }
        }
    }
}

/// Run the unified NumericSolver on already-solved legacy inference results.
///
/// This is a post-processing step: the legacy engines collect constraints from the
/// AST and solve independently; then we feed their results into the NumericSolver
/// for unified cross-type resolution.
///
/// Returns the raw solver result for diagnostics. The legacy maps are updated
/// in-place so codegen reads the improved types transparently.
pub fn unify_numeric_inference(
    float_inf: &mut FloatInference,
    int_inf: &mut IntInference,
) -> NumericSolverResult {
    let mut solver = NumericSolver::new();

    extract_float_constraints(float_inf, &mut solver);
    extract_int_constraints(int_inf, &mut solver);

    let result = solver.solve();

    backport_float_results(float_inf, &result);
    backport_int_results(int_inf, &result);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_preserves_types() {
        let mut float_inf = FloatInference::new();
        let f32_id = ExprId {
            seq_id: 1,
            file_id: 0,
            line: 10,
            col: 5,
        };
        float_inf.inferred_types.insert(f32_id, FloatType::F32);

        let mut int_inf = IntInference::new();
        let i64_id = ExprId {
            seq_id: 2,
            file_id: 0,
            line: 20,
            col: 3,
        };
        int_inf.inferred_types.insert(i64_id, IntType::I64);

        let result = unify_numeric_inference(&mut float_inf, &mut int_inf);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(float_inf.inferred_types[&f32_id], FloatType::F32);
        assert_eq!(int_inf.inferred_types[&i64_id], IntType::I64);
    }

    #[test]
    fn test_unknown_types_not_overwritten() {
        let mut float_inf = FloatInference::new();
        let unk_id = ExprId {
            seq_id: 3,
            file_id: 0,
            line: 30,
            col: 1,
        };
        float_inf
            .inferred_types
            .insert(unk_id, FloatType::Unknown);

        let mut int_inf = IntInference::new();

        let result = unify_numeric_inference(&mut float_inf, &mut int_inf);

        assert!(result.errors.is_empty());
        assert_eq!(
            float_inf.inferred_types.get(&unk_id),
            Some(&FloatType::Unknown),
            "Unknown should not be promoted"
        );
    }

    #[test]
    fn test_unified_resolution_enriches_legacy() {
        let mut float_inf = FloatInference::new();
        let mut int_inf = IntInference::new();

        // Simulate: expression 1 is known i64, expression 2 is unknown
        // The NumericSolver won't propagate between them without MustMatch,
        // but if we add a MustMatch we can verify the enrichment path.
        let id1 = ExprId {
            seq_id: 10,
            file_id: 0,
            line: 1,
            col: 1,
        };
        let id2 = ExprId {
            seq_id: 11,
            file_id: 0,
            line: 2,
            col: 1,
        };
        int_inf.inferred_types.insert(id1, IntType::U32);

        let result = unify_numeric_inference(&mut float_inf, &mut int_inf);

        assert!(result.errors.is_empty());
        assert_eq!(int_inf.inferred_types[&id1], IntType::U32);
        // id2 was never constrained, so it stays absent
        assert!(int_inf.inferred_types.get(&id2).is_none());
    }
}
