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
pub mod const_table;
pub mod float_inference;
pub mod int_conversion_table;
pub mod int_implicit_casts;
pub mod int_inference;
pub mod sig_table;
pub(crate) mod struct_field_registry;

pub use float_inference::{ExprId, FloatInference, FloatType};
pub use int_implicit_casts::{get_cast_suffix, is_safe_implicit_cast, promote_types};
pub use int_inference::{IntInference, IntType};

/// Final segment of a possibly module-qualified generic name (`std::collections::HashMap` → `HashMap`).
pub(crate) fn generic_type_base_name(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// A `use` path with a brace group, `dep::{A, B}`, as one path per imported name (`dep::A`, `dep::B`); any other path
/// as itself. The parser keeps a brace group inside one path segment.
pub(crate) fn expand_use_group(path: &[String]) -> Vec<Vec<String>> {
    let joined = path.join("::");
    let Some((prefix, group)) = joined.split_once("::{") else {
        return vec![path.to_vec()];
    };
    let Some(group) = group.strip_suffix('}') else {
        return vec![path.to_vec()];
    };
    if group.contains('{') {
        return vec![path.to_vec()];
    }
    group
        .split(',')
        .filter_map(|name| name.split_whitespace().next())
        .filter(|name| *name != "self")
        .map(|name| prefix.split("::").chain(std::iter::once(name)).map(str::to_string).collect())
        .collect()
}
