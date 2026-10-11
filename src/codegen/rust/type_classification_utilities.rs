//! Type Classification Utilities
//!
//! Helper functions for classifying and analyzing Windjammer types.
//! These are pure functions with no state dependencies.

use crate::parser::{Expression, Literal, Type};

/// Check if a type is an integer type (Int, Int32, Uint, or custom integer types).
pub fn is_integer_type(t: &Type) -> bool {
    match t {
        Type::Int | Type::Int32 | Type::Uint => true,
        Type::Custom(n) => crate::type_classification::is_integer_type(n),
        _ => false,
    }
}

/// Check if a type is a float type (Float, f32, or f64).
pub fn is_float_type(t: &Type) -> bool {
    match t {
        Type::Float => true,
        Type::Custom(n) => matches!(n.as_str(), "f32" | "f64"),
        _ => false,
    }
}

/// Determine the target float type for casting (f32 or f64).
/// Defaults to f32 unless explicitly f64.
pub fn float_target(t: &Type) -> &str {
    match t {
        Type::Custom(n) if n == "f64" => "f64",
        _ => "f32",
    }
}

/// Generate cast expression from integer to float.
/// Wraps binary expressions in parentheses before casting.
pub fn cast_int_to_float(s: &str, expr: &Expression, target: &str) -> String {
    if s.contains(" as ") || matches!(expr, Expression::Binary { .. }) {
        format!("({}) as {}", s, target)
    } else {
        format!("{} as {}", s, target)
    }
}

/// Strip partial literal-only float casts inside a binary expression string.
pub(crate) fn strip_partial_literal_float_casts(s: &str) -> String {
    s.replace("_i32 as f32", "_i32")
        .replace("_i64 as f64", "_i64")
        .replace("_i32 as f64", "_i32")
        .replace("_usize as f32", "_usize")
        .replace("_usize as f64", "_usize")
}

// Strip redundant same-width int casts (`0_i32 as i32` → `0_i32`) — .
pub fn strip_redundant_same_width_int_cast(expr: &mut String) {
    const PAIRS: &[(&str, &str)] = &[
        ("_i32", " as i32"),
        ("_i64", " as i64"),
        ("_u32", " as u32"),
        ("_u64", " as u64"),
        ("_usize", " as usize"),
    ];
    for &(suffix, cast) in PAIRS {
        if let Some(base) = expr.strip_suffix(cast) {
            if base.ends_with(suffix) {
                *expr = base.to_string();
                return;
            }
        }
    }
}

/// True when `s` is exactly `(<expr>) as <target>`, possibly wrapped in redundant parentheses:
/// the cast already covers the whole expression (an unparenthesized `a + b as f32` only casts
/// the last operand and does not count).
fn is_whole_expression_cast(s: &str, target: &str) -> bool {
    /// Index of the `)` matching the leading `(`, if `s` starts with one.
    fn matching_close(s: &str) -> Option<usize> {
        let mut depth = 0i32;
        for (i, c) in s.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }
    let t = s.trim();
    if let Some(inner) = t.strip_suffix(&format!(" as {target}")) {
        return inner.starts_with('(') && matching_close(inner) == Some(inner.len() - 1);
    }
    if t.starts_with('(') && matching_close(t) == Some(t.len() - 1) {
        return is_whole_expression_cast(&t[1..t.len() - 1], target);
    }
    false
}

/// Auto-cast an integer call argument to float when the parameter expects f32/f64.
///
/// Returns `true` (and mutates `arg_str`) if a cast was applied.
/// Skips if the arg string already contains ` as f32`/` as f64`.
pub fn maybe_cast_int_arg_to_float(
    arg_str: &mut String,
    arg_expr: &Expression,
    param_type: &Type,
    arg_type: Option<&Type>,
) -> bool {
    let param_is_f32 =
        matches!(param_type, Type::Float) || matches!(param_type, Type::Custom(n) if n == "f32");
    let param_is_f64 = matches!(param_type, Type::Custom(n) if n == "f64");
    if !param_is_f32 && !param_is_f64 {
        return false;
    }

    // Earlier coercion stages may already have wrapped the expression; never stack casts.
    for target in ["f32", "f64"] {
        let stacked = format!(" as {target}) as {target}");
        let single = format!(" as {target})");
        while arg_str.contains(&stacked) {
            *arg_str = arg_str.replace(&stacked, &single);
        }
    }
    // Binary int args need a whole-expression cast; partial inner literal casts break precedence.
    if matches!(arg_expr, Expression::Binary { .. }) {
        let target = if param_is_f32 { "f32" } else { "f64" };
        if is_whole_expression_cast(arg_str, target) {
            return false;
        }
        let normalized = strip_partial_literal_float_casts(arg_str);
        *arg_str = format!("({}) as {}", normalized, target);
        return true;
    }

    let arg_is_int = matches!(
        arg_expr,
        Expression::Literal {
            value: Literal::Int(_),
            ..
        }
    ) || arg_type.is_some_and(|t| {
        matches!(t, Type::Int | Type::Int32 | Type::Uint)
            || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
    });
    if !arg_is_int {
        return false;
    }
    if arg_str.contains(" as f32") || arg_str.contains(" as f64") {
        // Binary ops may have partial inner casts (e.g. `x + 1_i32 as f32`) — wrap whole expr.
        if matches!(arg_expr, Expression::Binary { .. }) {
            let target = if param_is_f32 { "f32" } else { "f64" };
            *arg_str = format!("({}) as {}", arg_str, target);
            return true;
        }
        return false;
    }

    let target = if param_is_f32 { "f32" } else { "f64" };
    *arg_str = cast_int_to_float(arg_str, arg_expr, target);
    true
}

/// Peel off the outermost reference layer from a type.
/// Returns the inner type if wrapped in Type::Reference, otherwise returns the type unchanged.
pub fn peel_reference_layer(t: &Type) -> &Type {
    match t {
        Type::Reference(inner) => inner.as_ref(),
        _ => t,
    }
}
