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

/// True when a formal is Rust `i32` (e.g. `process::exit`, FFI status codes).
pub fn type_is_i32(ty: &Type) -> bool {
    matches!(ty, Type::Int32) || matches!(ty, Type::Custom(n) if n == "i32")
}

/// Drive int literal suffixes while generating a call argument (P3.353 `set_if` coords).
pub fn assignment_int_peer_from_formal(formal: Option<&Type>) -> Option<Type> {
    if formal.is_some_and(type_is_i32) {
        return Some(Type::Int32);
    }
    if formal.is_some_and(|ty| {
        matches!(ty, Type::Uint) || matches!(ty, Type::Custom(n) if n == "u32")
    }) {
        return Some(Type::Uint);
    }
    None
}

/// Coerce a WJ `int`/`i64` argument to an `i32` formal (WDB-160 / `process::exit`).
///
/// Signature-driven: only when the resolved formal is `i32`. Literals already
/// suffixed `_i32` are left alone; identifiers/`i64` bindings get `as i32`.
/// Skip when the argument is already `i32` — avoids `quantity as i32` noise.
pub fn coerce_arg_str_for_i32_formal(
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
    arg_type: Option<&Type>,
) {
    if !formal.is_some_and(type_is_i32) {
        return;
    }
    if arg_type.is_some_and(type_is_i32) && !arg_str.contains("_i64") {
        return;
    }
    if arg_str.contains(" as i32") || arg_str.ends_with("_i32") {
        return;
    }
    // Already a narrow int literal without suffix is fine for Rust inference,
    // but WJ defaults emit `_i64` — strip and re-suffix, or cast bindings.
    if let Expression::Literal {
        value: Literal::Int(val),
        ..
    } = arg
    {
        let i64_suf = format!("{val}_i64");
        if *arg_str == i64_suf || *arg_str == val.to_string() {
            *arg_str = format!("{val}_i32");
            return;
        }
    }
    let needs_parens = matches!(arg, Expression::Binary { .. }) || arg_str.contains(' ');
    if needs_parens {
        *arg_str = format!("({}) as i32", arg_str);
    } else {
        *arg_str = format!("{} as i32", arg_str);
    }
}

/// True when a formal is Rust `u32` / WJ `uint`.
pub fn type_is_u32(ty: &Type) -> bool {
    matches!(ty, Type::Uint) || matches!(ty, Type::Custom(n) if n == "u32")
}

/// WJ `int` / Rust `i64` formals (entity ids, etc.).
pub fn type_is_wj_i64(ty: &Type) -> bool {
    matches!(ty, Type::Int) || matches!(ty, Type::Custom(n) if n == "int" || n == "i64")
}

fn arg_str_emits_i32(arg_str: &str) -> bool {
    arg_str.ends_with("_i32") || arg_str.contains(" as i32")
}

fn arg_str_emits_u32(arg_str: &str) -> bool {
    arg_str.ends_with("_u32") || arg_str.contains(" as u32")
}

fn arg_str_emits_i64(arg_str: &str) -> bool {
    arg_str.ends_with("_i64") || arg_str.contains(" as i64")
}

fn append_int_cast(arg: &Expression, arg_str: &mut String, suffix: &str) {
    if arg_str.contains(&format!(" as {suffix}")) {
        return;
    }
    if let Expression::Literal {
        value: Literal::Int(val),
        ..
    } = arg
    {
        if arg_str.ends_with("_i32") {
            *arg_str = format!("{val}_{suffix}");
            return;
        }
        if suffix == "i64" && arg_str.ends_with("_i64") {
            return;
        }
        if suffix == "u32" && arg_str.ends_with("_u32") {
            return;
        }
    }
    let needs_parens = matches!(arg, Expression::Binary { .. }) || arg_str.contains(' ');
    if needs_parens {
        *arg_str = format!("({}) as {}", arg_str, suffix);
    } else {
        *arg_str = format!("{} as {}", arg_str, suffix);
    }
}

/// P3.368: i32-coord locals/literals into `i64` / WJ `int` formals (ECS entity ids).
pub fn coerce_arg_str_for_i64_formal(
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
    arg_type: Option<&Type>,
    mixed_int: Option<crate::type_inference::IntType>,
) {
    if !formal.is_some_and(type_is_wj_i64) {
        return;
    }
    if arg_type.is_some_and(type_is_wj_i64) || arg_str_emits_i64(arg_str) {
        return;
    }
    if arg_type.is_some_and(type_is_i32)
        || arg_str_emits_i32(arg_str)
        || mixed_int == Some(crate::type_inference::IntType::I32)
    {
        append_int_cast(arg, arg_str, "i64");
    }
}

/// P3.368: i32-coord locals/literals into `u32` formals (GPU / texture FFI).
pub fn coerce_arg_str_for_u32_formal(
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
    arg_type: Option<&Type>,
    mixed_int: Option<crate::type_inference::IntType>,
) {
    if !formal.is_some_and(type_is_u32) {
        return;
    }
    if arg_type.is_some_and(type_is_u32) || arg_str_emits_u32(arg_str) {
        return;
    }
    if arg_type.is_some_and(type_is_i32)
        || arg_str_emits_i32(arg_str)
        || mixed_int == Some(crate::type_inference::IntType::I32)
    {
        append_int_cast(arg, arg_str, "u32");
    }
}

/// All signature-driven numeric formal coercions (extern + IR terminal).
pub fn apply_numeric_formal_coercions(
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
    arg_type: Option<&Type>,
    mixed_int: Option<crate::type_inference::IntType>,
) {
    coerce_arg_str_for_usize_formal(None, arg, arg_str, formal, false);
    coerce_arg_str_for_i32_formal(arg, arg_str, formal, arg_type);
    coerce_arg_str_for_i64_formal(arg, arg_str, formal, arg_type, mixed_int);
    coerce_arg_str_for_u32_formal(arg, arg_str, formal, arg_type, mixed_int);
}

/// P3.368: struct literal field slot — i32 binding into u32/i64 field.
pub fn coerce_struct_field_numeric(
    arg: &Expression,
    expr_str: &mut String,
    field_type: &Type,
    arg_type: Option<&Type>,
) {
    if type_is_u32(field_type) {
        coerce_arg_str_for_u32_formal(arg, expr_str, Some(field_type), arg_type, None);
    } else if type_is_wj_i64(field_type) {
        coerce_arg_str_for_i64_formal(arg, expr_str, Some(field_type), arg_type, None);
    }
}


/// Coerce a call argument to match a `usize` formal (Rust collection capacity/index).
///
/// Signature-driven: only runs when the resolved formal is `usize`. Skips when the
/// argument is already usize-typed or already lowered with `as usize` / `_usize`.
/// Undo a spurious `1_usize` / `n as usize` when the resolved formal is not `usize`
/// (e.g. numeric inference applied Vec::insert to a HashMap key before codegen).
fn expression_must_not_usize_coerce(
    gen: Option<&crate::codegen::rust::CodeGenerator<'_>>,
    arg: &Expression,
    arg_str: &str,
) -> bool {
    if arg_str.contains(".to_string()")
        || arg_str.contains("String::")
        || arg_str.contains("format!(")
    {
        return true;
    }
    if matches!(
        arg,
        Expression::Literal {
            value: Literal::String(_),
            ..
        } | Expression::StructLiteral { .. }
            | Expression::Tuple { .. }
    ) {
        return true;
    }
    let Some(gen) = gen else {
        return false;
    };
    if gen
        .infer_expression_type(arg)
        .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(&t))
    {
        return true;
    }
    if let Expression::Binary {
        op: crate::parser::BinaryOp::Add,
        left,
        right,
        ..
    } = arg
    {
        return [left, right].iter().any(|e| {
            gen.infer_expression_type(e)
                .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(&t))
        });
    }
    false
}

pub fn strip_erroneous_usize_suffix_for_non_usize_formal(
    gen: Option<&crate::codegen::rust::CodeGenerator<'_>>,
    arg: &Expression,
    arg_str: &mut String,
    formal: Option<&Type>,
) {
    if formal.is_some_and(type_is_usize) {
        return;
    }
    if expression_must_not_usize_coerce(gen, arg, arg_str) {
        if let Some(base) = arg_str.strip_suffix(" as usize") {
            *arg_str = base.to_string();
        } else if let Some(base) = arg_str
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(" as usize)"))
        {
            *arg_str = base.to_string();
        }
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
    gen: Option<&crate::codegen::rust::CodeGenerator<'_>>,
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
    if expression_must_not_usize_coerce(gen, arg, arg_str) {
        return;
    }
    // Whole-expression casts are done. Do **not** treat `i + j + 1_usize` as already
    // usize — that mixes i64 counters with a usize literal suffix (haystack / LedgerKit).
    if arg_str.contains(" as usize") {
        return;
    }
    if arg_str.ends_with("_usize") {
        if matches!(
            arg,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return;
        }
        // Binary / nested expr with a mid-tree `_usize` literal: normalize then cast.
        let cleaned = strip_embedded_usize_literal_suffixes(arg_str);
        *arg_str = format!("({cleaned}) as usize");
        return;
    }
    // Never usize-cast text / constructed values. A wrong suffix signature
    // (Vec::insert vs HashMap::insert) must not turn `"key".to_string()` into usize.
    // Struct / tuple literals into owned Custom formals must never become `as usize`
    // (WDB-133 `drain(DenseCsr { n: 3 })`).
    if matches!(
        arg,
        Expression::Literal {
            value: Literal::String(_),
            ..
        } | Expression::StructLiteral { .. }
            | Expression::Tuple { .. }
    ) || arg_str.contains(".to_string()")
        || arg_str.contains("String::")
        || arg_str.contains("format!(")
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
            // Owned `usize` formals take by value — peel stale `&` / `&mut` from
            // reuse / mut-local tracking (`&mut i as usize` is E0606).
            let base = arg_str
                .strip_prefix("&mut ")
                .or_else(|| arg_str.strip_prefix('&'))
                .unwrap_or(arg_str);
            *arg_str = format!("{base} as usize");
        }
        _ => {
            *arg_str = format!("({arg_str}) as usize");
        }
    }
}

/// Peel `N_usize` → `N` inside compound expressions so the outer `(…) as usize` is valid.
fn strip_embedded_usize_literal_suffixes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let num = &s[start..i];
            if s[i..].starts_with("_usize") {
                // Only rewrite when the digit run is a bare literal, not `x1_usize`.
                let prev_ok = start == 0
                    || matches!(
                        bytes[start - 1],
                        b'(' | b'[' | b' ' | b'+' | b'-' | b'*' | b'/' | b'%' | b',' | b'='
                    );
                if prev_ok {
                    out.push_str(num);
                    i += "_usize".len();
                    continue;
                }
            }
            out.push_str(&s[start..i]);
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
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

    #[test]
    fn coerce_usize_formal_wraps_mixed_i64_plus_usize_literal() {
        use crate::parser::{Expression, Literal, Type};
        // Approximate Binary `i + j + 1` lowered as `i + j + 1_usize`.
        let arg = Expression::Literal {
            value: Literal::Int(1),
            location: Default::default(),
        };
        // Use a non-literal expr shape via Identifier so we hit the binary path —
        // the AST kind gates the early-return; the string is what we normalize.
        let arg = Expression::Identifier {
            name: "i".into(),
            location: Default::default(),
        };
        let mut s = "i + j + 1_usize".to_string();
        coerce_arg_str_for_usize_formal(
            None,
            &arg,
            &mut s,
            Some(&Type::Custom("usize".into())),
            false,
        );
        assert_eq!(s, "(i + j + 1) as usize");
        let _ = Literal::Int(0);
    }
}
