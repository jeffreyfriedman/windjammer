//! Target-agnostic call-site coercion decisions.
//!
//! Computes `(actual SafetyType, expected SafetyType) → CoercionKind` without
//! method-name heuristics or backend-specific string manipulation. Backends apply
//! the result via `target_encodings::apply_coercion`.

use crate::ir::safety_type::{BaseType, OwnedType, SafetyType};

/// Target-agnostic coercion applied to a generated expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoercionKind {
    /// No transformation needed.
    Identity,
    /// Pass by shared reference (owned → borrowed).
    Borrow,
    /// Pass by mutable reference.
    MutBorrow,
    /// Clone to obtain owned value from reference.
    Clone,
    /// Deref copy type from reference (Rust `*`).
    Deref,
    /// String literal or &str → owned string.
    ToOwnedString,
    /// Remove spurious borrow (Copy formal, owned pass-by-value).
    StripBorrow,
    /// Numeric cast to expected base type.
    NumericCast(BaseType),
}

/// Compute the coercion needed to pass `actual` where `expected` is required.
///
/// This is the single decision point for call-site lowering across all backends.
pub fn compute_coercion(actual: &SafetyType, expected: &SafetyType) -> CoercionKind {
    let actual_own = normalize_ownership(&actual.ownership);
    let expected_own = normalize_ownership(&expected.ownership);

    // String literals are &str-shaped in Rust; never prefix `&` (would produce &&str).
    if is_string_base(&actual.base) && matches!(actual_own, OwnedType::Ref(_)) {
        return match expected_own {
            OwnedType::Owned => CoercionKind::ToOwnedString,
            OwnedType::Ref(_) | OwnedType::Copy => CoercionKind::Identity,
            OwnedType::MutRef(_) => CoercionKind::ToOwnedString,
            OwnedType::Inferred => CoercionKind::Identity,
        };
    }

    // Copy types: pass by value; strip spurious borrows when callee expects owned/copy.
    if matches!(expected_own, OwnedType::Copy | OwnedType::Owned)
        && matches!(actual_own, OwnedType::Ref(_))
        && is_copy_base(&expected.base)
    {
        return CoercionKind::StripBorrow;
    }

    if matches!(expected_own, OwnedType::Copy) && actual_own == OwnedType::Owned {
        return CoercionKind::Identity;
    }

    // Same copy base (e.g. f32 literal → f32 param): no cast.
    if matches!(expected_own, OwnedType::Copy | OwnedType::Owned)
        && matches!(actual_own, OwnedType::Copy | OwnedType::Owned)
        && actual.base == expected.base
    {
        return CoercionKind::Identity;
    }

    // Mutable borrow required.
    if matches!(expected_own, OwnedType::MutRef(_)) {
        return match actual_own {
            OwnedType::MutRef(_) => CoercionKind::Identity,
            OwnedType::Ref(_) => CoercionKind::MutBorrow,
            OwnedType::Owned | OwnedType::Copy => CoercionKind::MutBorrow,
            OwnedType::Inferred => CoercionKind::MutBorrow,
        };
    }

    // Shared borrow required.
    if matches!(expected_own, OwnedType::Ref(_)) {
        return match actual_own {
            OwnedType::Ref(_) => CoercionKind::Identity,
            OwnedType::MutRef(_) => CoercionKind::Borrow,
            OwnedType::Copy => CoercionKind::Borrow,
            OwnedType::Owned => {
                if is_string_base(&expected.base)
                    || is_string_base(&actual.base)
                    || is_vec_base(&expected.base)
                    || is_vec_base(&actual.base)
                {
                    CoercionKind::Borrow
                } else if matches!(
                    (&actual.base, &expected.base),
                    (BaseType::Custom(_), BaseType::Custom(_))
                ) {
                    // Owned Custom → `&T` formal: emit explicit `&` (WDB-039 keys_equal,
                    // Copy map/set keys). Rust auto-ref alone leaves bare locals that fail
                    // dogfooding assertions and asymmetric clone lowering.
                    CoercionKind::Borrow
                } else {
                    CoercionKind::Borrow
                }
            }
            OwnedType::Inferred => CoercionKind::Borrow,
        };
    }

    // Owned expected.
    if matches!(expected_own, OwnedType::Owned) {
        return match actual_own {
            OwnedType::Owned | OwnedType::Copy => {
                if needs_string_owned_coercion(&actual.base, &expected.base) {
                    CoercionKind::ToOwnedString
                } else if needs_numeric_cast(&actual.base, &expected.base) {
                    CoercionKind::NumericCast(expected.base.clone())
                } else {
                    CoercionKind::Identity
                }
            }
            OwnedType::Ref(_) => {
                if is_string_base(&expected.base) {
                    CoercionKind::ToOwnedString
                } else if is_copy_base(&actual.base) {
                    CoercionKind::Deref
                } else {
                    CoercionKind::Clone
                }
            }
            OwnedType::MutRef(_) => CoercionKind::StripBorrow,
            OwnedType::Inferred => CoercionKind::Identity,
        };
    }

    CoercionKind::Identity
}

/// Apply the IR ownership contract to emitted Rust call argument text.
pub fn enforce_ownership_contract_on_coerced_arg(
    coerced: &mut String,
    actual: &SafetyType,
    expected: &SafetyType,
) {
    enforce_ownership_contract_on_coerced_arg_with_force_owned(coerced, actual, expected, false, false, false);
}

fn strip_rust_ref_expr(expr: &str) -> &str {
    let trimmed = expr.trim();
    let inner = if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed[1..trimmed.len() - 1].trim()
    } else {
        trimmed
    };
    inner
        .trim_start_matches("&mut ")
        .trim_start_matches('&')
        .trim()
}

pub fn enforce_ownership_contract_on_coerced_arg_with_force_owned(
    coerced: &mut String,
    actual: &SafetyType,
    expected: &SafetyType,
    force_owned_contract: bool,
    allow_rust_auto_borrow: bool,
    preserve_runtime_std_borrow: bool,
) {
    if !preserve_runtime_std_borrow
        && (matches!(expected.ownership, OwnedType::Owned) || force_owned_contract)
    {
        let (is_ref, inner) = if coerced.starts_with("&mut ") {
            (true, coerced["&mut ".len()..].trim())
        } else if coerced.starts_with('&') {
            (true, coerced[1..].trim())
        } else if coerced.starts_with('(') && coerced.ends_with(')') {
            let inner_expr = coerced[1..coerced.len() - 1].trim();
            if inner_expr.starts_with("&mut ") {
                (true, inner_expr["&mut ".len()..].trim())
            } else if inner_expr.starts_with('&') {
                (true, inner_expr[1..].trim())
            } else {
                (false, coerced.as_str())
            }
        } else {
            (false, coerced.as_str())
        };
            if is_ref && !coerced.starts_with("&mut ") {
            let inner = inner
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(inner);
            if inner.starts_with('*') {
                *coerced = inner.to_string();
            } else if is_copy_base(&expected.base)
                && !matches!(expected.base, BaseType::Custom(_))
            {
                *coerced = format!("*{inner}");
            } else {
                *coerced = format!("{inner}.clone()");
            }
            return;
        }
    }
    if allow_rust_auto_borrow
        && coerced.starts_with('&')
        && !coerced.starts_with("&mut ")
    {
        *coerced = coerced.trim_start_matches('&').to_string();
        return;
    }
    let kind = compute_coercion(actual, expected);
    if allow_rust_auto_borrow
        && matches!(kind, CoercionKind::Identity)
        && matches!(expected.ownership, OwnedType::Ref(_))
        && coerced.starts_with('&')
        && !coerced.starts_with("&mut ")
    {
        *coerced = coerced.trim_start_matches('&').to_string();
        return;
    }
    if matches!(expected.ownership, OwnedType::Ref(_))
        && matches!(kind, CoercionKind::Borrow)
        && !coerced.starts_with('&')
        && !coerced.starts_with("&mut ")
        // Rust string literals are already `&str`; prefixing `&` yields `&&str`.
        && !crate::codegen::rust::expression_utilities::is_rust_string_literal_text(coerced)
    {
        *coerced = format!("&{coerced}");
        return;
    }
    // Ref binding → owned formal: deref Copy (`*through`) or clone non-Copy.
    // Text `&x` was handled above (strip). Bare `through` where through: &T needs this.
    // Skip when the Rust text is already a field/index projection (`failure.status`) —
    // those Copy projections are values, not places that need `*`.
    if matches!(
        expected.ownership,
        OwnedType::Owned | OwnedType::Copy
    ) && matches!(actual.ownership, OwnedType::Ref(_))
        && !coerced.starts_with('&')
        && !coerced.starts_with('*')
        && !coerced.contains('.')
        && !coerced.contains('[')
    {
        match kind {
            CoercionKind::Deref | CoercionKind::StripBorrow => {
                // StripBorrow on bare ref bindings is a no-op; prefer deref for Copy,
                // clone otherwise (safe for aggregates).
                if is_copy_base(&expected.base) || is_copy_base(&actual.base) {
                    let core = strip_rust_ref_expr(coerced);
                    *coerced = format!("*{core}");
                } else if matches!(kind, CoercionKind::Deref) {
                    let base = strip_rust_ref_expr(coerced);
                    *coerced = format!("{base}.clone()");
                } else if !coerced.ends_with(".clone()") {
                    let base = strip_rust_ref_expr(coerced);
                    *coerced = format!("{base}.clone()");
                }
            }
            CoercionKind::Clone => {
                if !coerced.ends_with(".clone()")
                    && !coerced.ends_with(".to_string()")
                    && !coerced.ends_with(".to_owned()")
                {
                    let base = strip_rust_ref_expr(coerced);
                    *coerced = format!("{base}.clone()");
                }
            }
            _ => {}
        }
    }
}

fn normalize_ownership(own: &OwnedType) -> OwnedType {
    match own {
        OwnedType::Copy => OwnedType::Owned,
        other => other.clone(),
    }
}

fn is_copy_base(base: &BaseType) -> bool {
    match base {
        BaseType::Bool
        | BaseType::Char
        | BaseType::I8
        | BaseType::I16
        | BaseType::I32
        | BaseType::I64
        | BaseType::I128
        | BaseType::U8
        | BaseType::U16
        | BaseType::U32
        | BaseType::U64
        | BaseType::U128
        | BaseType::F32
        | BaseType::F64 => true,
        BaseType::Tuple(elems) => elems.iter().all(is_copy_base),
        _ => false,
    }
}

fn is_integer_base(base: &BaseType) -> bool {
    matches!(
        base,
        BaseType::I8
            | BaseType::I16
            | BaseType::I32
            | BaseType::I64
            | BaseType::I128
            | BaseType::U8
            | BaseType::U16
            | BaseType::U32
            | BaseType::U64
            | BaseType::U128
    )
}

fn is_float_base(base: &BaseType) -> bool {
    matches!(base, BaseType::F32 | BaseType::F64)
}

pub(crate) fn is_string_base(base: &BaseType) -> bool {
    matches!(base, BaseType::String)
}

pub(crate) fn is_vec_base(base: &BaseType) -> bool {
    matches!(base, BaseType::Custom(name) if name == "Vec")
}

fn needs_string_owned_coercion(actual: &BaseType, expected: &BaseType) -> bool {
    is_string_base(expected) && is_string_base(actual)
}

fn needs_numeric_cast(actual: &BaseType, expected: &BaseType) -> bool {
    if actual == expected {
        return false;
    }
    // f32 and f64 are compatible without explicit cast when expr already has suffix.
    if is_float_base(actual) && is_float_base(expected) {
        return false;
    }
    is_integer_base(actual) && is_float_base(expected)
        || is_float_base(actual) && is_integer_base(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::safety_type::Region;

    fn owned(base: BaseType) -> SafetyType {
        SafetyType::owned(base)
    }

    fn borrowed(base: BaseType) -> SafetyType {
        SafetyType::borrowed(base, Region::fresh(0))
    }

    fn mut_borrowed(base: BaseType) -> SafetyType {
        SafetyType::mut_borrowed(base, Region::fresh(1))
    }

    fn copy(base: BaseType) -> SafetyType {
        SafetyType::copy(base)
    }

    #[test]
    fn string_literal_to_ref_param_is_identity_not_borrow() {
        let actual = borrowed(BaseType::String);
        let expected = borrowed(BaseType::String);
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Identity);
    }

    #[test]
    fn string_literal_to_owned_string_needs_to_owned() {
        let actual = borrowed(BaseType::String);
        let expected = owned(BaseType::String);
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::ToOwnedString);
    }

    #[test]
    fn owned_custom_struct_to_ref_param_is_identity_auto_borrow() {
        let actual = owned(BaseType::Custom("Key".into()));
        let expected = borrowed(BaseType::Custom("Key".into()));
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Identity);
    }

    #[test]
    fn owned_copy_struct_to_ref_param_needs_borrow() {
        let actual = copy(BaseType::Custom("Lsn".into()));
        let expected = borrowed(BaseType::Custom("Lsn".into()));
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Borrow);
    }

    #[test]
    fn owned_vec_to_ref_param_needs_borrow() {
        let actual = owned(BaseType::Custom("Vec".into()));
        let expected = borrowed(BaseType::Custom("Vec".into()));
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Borrow);
    }

    #[test]
    fn borrowed_to_owned_string_needs_to_owned() {
        let actual = borrowed(BaseType::String);
        let expected = owned(BaseType::String);
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::ToOwnedString);
    }

    #[test]
    fn borrowed_to_owned_string_needs_clone_non_string() {
        let actual = borrowed(BaseType::Custom("Vec".into()));
        let expected = owned(BaseType::Custom("Vec".into()));
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Clone);
    }

    #[test]
    fn copy_formal_with_spurious_borrow_strips() {
        let actual = borrowed(BaseType::I32);
        let expected = copy(BaseType::I32);
        assert_eq!(
            compute_coercion(&actual, &expected),
            CoercionKind::StripBorrow
        );
    }

    #[test]
    fn owned_to_mut_borrowed_needs_mut_borrow() {
        let actual = owned(BaseType::Custom("Vec".into()));
        let expected = mut_borrowed(BaseType::Custom("Vec".into()));
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::MutBorrow);
    }

    #[test]
    fn borrowed_copy_tuple_to_owned_strips_or_derefs() {
        let tuple = BaseType::Tuple(vec![BaseType::I32, BaseType::I32]);
        let actual = borrowed(tuple.clone());
        let expected = owned(tuple);
        // Owned Copy formals: StripBorrow (auto-copy) is preferred over explicit Deref.
        assert_eq!(
            compute_coercion(&actual, &expected),
            CoercionKind::StripBorrow
        );
    }

    #[test]
    fn copy_pass_through_identity() {
        let actual = copy(BaseType::I32);
        let expected = copy(BaseType::I32);
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Identity);
    }

    #[test]
    fn f32_literal_to_f32_param_identity() {
        let actual = copy(BaseType::F32);
        let expected = copy(BaseType::F32);
        assert_eq!(compute_coercion(&actual, &expected), CoercionKind::Identity);
    }

    #[test]
    fn int_to_float_needs_numeric_cast() {
        let actual = owned(BaseType::I32);
        let expected = owned(BaseType::F64);
        assert_eq!(
            compute_coercion(&actual, &expected),
            CoercionKind::NumericCast(BaseType::F64)
        );
    }
}
