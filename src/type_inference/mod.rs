/// Expression-Level Float Type Inference
///
/// GOAL: Infer f32 vs f64 for each float literal based on usage context
///
/// ALGORITHM: Constraint-based unification
/// 1. Collect constraints from expressions (x: f32 + 0.0 → 0.0 must be f32)
/// 2. Propagate constraints through AST (forward and backward)
/// 3. Unify constraints (all uses of a literal must agree)
/// 4. Detect conflicts (f32 * f64 → Windjammer error)
/// 5. Assign final types (each literal gets f32 or f64)

pub mod float_inference;
pub mod int_inference;
pub mod int_implicit_casts;

pub use float_inference::{FloatInference, FloatType, ExprId};
pub use int_inference::{IntInference, IntType};
pub use int_implicit_casts::{is_safe_implicit_cast, get_cast_suffix, promote_types};
