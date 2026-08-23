// Auto-casting utilities for type conversions
// Extracted from generator.rs for better modularity

use crate::parser::ast::*;

/// Check if an expression is a usize literal
pub fn is_usize_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Literal {
            value: Literal::Int(_),
            ..
        }
    )
}

/// Generate a cast from usize to i64 if needed
pub fn maybe_cast_usize_to_int(expr_str: String, needs_cast: bool) -> String {
    if needs_cast {
        format!("({} as i64)", expr_str)
    } else {
        expr_str
    }
}

/// True when a WJ/parser type is the Rust `usize` capacity/index formal.
pub fn type_is_usize(ty: &Type) -> bool {
    matches!(ty, Type::Custom(n) if n == "usize")
}

/// WJ `int` / `i64` formals that may lower to runtime `usize` via fallback registry.
pub fn type_is_wj_int_formal(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::Int32)
        || matches!(ty, Type::Custom(n) if n == "int" || n == "i64" || n == "i32")
}

/// Coerce a call argument to match a `usize` formal (Rust collection capacity/index).
///
/// Signature-driven: only runs when the resolved formal is `usize`. Skips when the
/// argument is already usize-typed or already lowered with `as usize` / `_usize`.
/// Undo a spurious `1_usize` / `n as usize` when the resolved formal is not `usize`
/// (e.g. numeric inference applied Vec::insert to a HashMap key before codegen).
pub fn strip_erroneous_usize_suffix_for_non_usize_formal(
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
) {
    if formal.is_some_and(type_is_usize) {
        return;
    }
    match arg {
        Expression::Literal {
            value: Literal::Int(val),
            ..
        } => {
            let expected = format!("{val}_usize");
            if *arg_str == expected {
                *arg_str = val.to_string();
            }
        }
        Expression::Identifier { .. } => {
            if let Some(base) = arg_str.strip_suffix(" as usize") {
                *arg_str = base.to_string();
            }
        }
        _ => {
            if let Some(base) = arg_str
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(" as usize)"))
            {
                *arg_str = base.to_string();
            } else if let Some(base) = arg_str.strip_suffix(" as usize") {
                *arg_str = base.to_string();
            }
        }
    }
}

pub fn coerce_arg_str_for_usize_formal(
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
    arg_already_usize: bool,
) {
    if !formal.is_some_and(type_is_usize) {
        return;
    }
    if arg_already_usize {
        return;
    }
    if arg_str.contains(" as usize") || arg_str.ends_with("_usize") {
        return;
    }
    // Never usize-cast text / constructed values. A wrong suffix signature
    // (Vec::insert vs HashMap::insert) must not turn `"key".to_string()` into usize.
    if matches!(
        arg,
        Expression::Literal {
            value: Literal::String(_),
            ..
        }
    ) || arg_str.contains(".to_string()")
        || arg_str.contains("String::")
    {
        return;
    }
    // Strip a stale signed cast from IR when WJ used to declare int capacity
    // (`n as i64` → `n as usize` for Rust HashMap::with_capacity).
    if let Some(base) = arg_str.strip_suffix(" as i64") {
        *arg_str = format!("{base} as usize");
        return;
    }
    if let Some(base) = arg_str
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(" as i64)"))
    {
        *arg_str = format!("({base}) as usize");
        return;
    }
    match arg {
        Expression::Literal {
            value: Literal::Int(val),
            ..
        } => {
            *arg_str = format!("{val}_usize");
        }
        Expression::Identifier { .. } => {
            *arg_str = format!("{arg_str} as usize");
        }
        _ => {
            *arg_str = format!("({arg_str}) as usize");
        }
    }
}

/// Generate a cast for usize in binary operations
pub fn cast_for_usize_binary_op(
    left_str: &str,
    right_str: &str,
    left_is_usize: bool,
    right_is_usize: bool,
) -> (String, String) {
    match (left_is_usize, right_is_usize) {
        (true, false) => {
            // Cast left (usize) to match right (int)
            (format!("({} as i64)", left_str), right_str.to_string())
        }
        (false, true) => {
            // Cast right (usize) to match left (int)
            (left_str.to_string(), format!("({} as i64)", right_str))
        }
        _ => {
            // No casting needed
            (left_str.to_string(), right_str.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maybe_cast_usize_to_int() {
        assert_eq!(
            maybe_cast_usize_to_int("vec.len()".to_string(), true),
            "(vec.len() as i64)"
        );
        assert_eq!(maybe_cast_usize_to_int("42".to_string(), false), "42");
    }

    #[test]
    fn test_cast_for_usize_binary_op() {
        let (left, right) = cast_for_usize_binary_op("vec.len()", "10", true, false);
        assert_eq!(left, "(vec.len() as i64)");
        assert_eq!(right, "10");

        let (left, right) = cast_for_usize_binary_op("10", "vec.len()", false, true);
        assert_eq!(left, "10");
        assert_eq!(right, "(vec.len() as i64)");

        let (left, right) = cast_for_usize_binary_op("x", "y", false, false);
        assert_eq!(left, "x");
        assert_eq!(right, "y");
    }
}
