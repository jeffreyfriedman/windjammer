//! Type-directed argument lowering.
//!
//! Replaces the heuristic-heavy sequential pipeline in `arguments.rs` with a single
//! decision function that computes the correct coercion for each argument based on:
//!
//! - The **formal** parameter type (what the generated Rust def actually says)
//! - The effective ownership mode (from signature resolution)
//! - Whether the formal type is Copy
//! - The shape of the argument expression (literal, identifier, field access, etc.)
//!
//! The core invariant: **one call to `compute_arg_coercion` per argument, one coercion
//! applied**. No sequential phases that add-then-strip-then-re-add.

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::codegen::rust::call_signature_resolution::effective_param_ownership;
use crate::codegen::rust::string_utilities;
use crate::codegen::rust::types;
use crate::ir::coercion::{compute_coercion, CoercionKind};
use crate::ir::signature_bridge::safety_type_from_signature_param;
use crate::ir::safety_type::{BaseType, Region, SafetyType};
use crate::parser::{Expression, Literal, Type};

/// What coercion to apply to a generated argument expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgCoercion {
    /// No transformation needed.
    PassThrough,
    /// Add `&` prefix (owned T -> &T).
    Borrow,
    /// Add `&mut` prefix (owned T -> &mut T).
    MutBorrow,
    /// Add `.clone()` suffix (&T -> owned T, non-Copy).
    Clone,
    /// Add `*` prefix (&T -> owned T, Copy).
    Deref,
    /// Add `.to_string()` suffix (str literal -> owned String).
    ToOwnedString,
    /// Add `&` prefix for String -> &str coercion.
    BorrowString,
    /// Strip a leading `&` that was explicitly written but is unnecessary.
    StripRef,
    /// Numeric cast, e.g. `as f64`.
    CastNumeric(String),
}

impl ArgCoercion {
    /// Apply this coercion to a generated Rust expression string.
    pub fn apply(&self, expr: &mut String) {
        match self {
            ArgCoercion::PassThrough => {}
            ArgCoercion::Borrow => {
                if !expr.starts_with('&') {
                    *expr = format!("&{}", expr);
                }
            }
            ArgCoercion::MutBorrow => {
                if expr.starts_with("&mut ") {
                    return;
                }
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(expr);
                *expr = format!("&mut {base}");
            }
            ArgCoercion::Clone => {
                if !expr.ends_with(".clone()") {
                    *expr = format!("{}.clone()", expr);
                }
            }
            ArgCoercion::Deref => {
                if !expr.starts_with('*') {
                    *expr = format!("*{}", expr);
                }
            }
            ArgCoercion::ToOwnedString => {
                if !expr.ends_with(".to_string()") && !expr.ends_with(".to_owned()") {
                    *expr = format!("{}.to_string()", expr);
                }
            }
            ArgCoercion::BorrowString => {
                if !expr.starts_with('&') {
                    *expr = format!("&{}", expr);
                }
            }
            ArgCoercion::StripRef => {
                if expr.starts_with("&mut ") {
                    *expr = expr["&mut ".len()..].to_string();
                } else if expr.starts_with('&') {
                    *expr = expr[1..].to_string();
                }
            }
            ArgCoercion::CastNumeric(target) => {
                *expr = format!("{} as {}", expr, target);
            }
        }
    }
}

/// Context for a single argument at a call site.
pub struct ArgContext<'a> {
    /// The function signature resolved for the callee.
    pub sig: &'a FunctionSignature,
    /// The call-site argument index (0-based, excludes implicit self).
    pub arg_index: usize,
    /// The AST expression for this argument.
    pub arg_expr: &'a Expression<'a>,
    /// The already-generated Rust expression string (before coercion).
    pub arg_str: &'a str,
    /// Whether the formal param type is Copy.
    pub is_formal_copy: bool,
    /// Whether the generated arg string already starts with `&`.
    pub arg_already_ref: bool,
    /// Whether the argument is a string literal.
    pub is_string_literal: bool,
    /// Whether the argument is a copy literal (int/float/bool).
    pub is_copy_literal: bool,
    /// Whether this is a collection key argument (HashMap::get arg 0, etc.).
    pub is_collection_key: bool,
    /// The method/function name being called.
    pub method_name: &'a str,
}

/// Whether to use the shared IR coercion engine inside `compute_arg_coercion`.
pub fn use_ir_coercion_engine() -> bool {
    std::env::var("WJ_IR_COERCION_ENGINE")
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn coercion_kind_to_arg(kind: CoercionKind) -> ArgCoercion {
    match kind {
        CoercionKind::Identity => ArgCoercion::PassThrough,
        CoercionKind::Borrow => ArgCoercion::Borrow,
        CoercionKind::MutBorrow => ArgCoercion::MutBorrow,
        CoercionKind::Clone => ArgCoercion::Clone,
        CoercionKind::Deref => ArgCoercion::Deref,
        CoercionKind::ToOwnedString => ArgCoercion::ToOwnedString,
        CoercionKind::StripBorrow => ArgCoercion::StripRef,
        CoercionKind::NumericCast(base) => {
            let cast = match base {
                BaseType::F32 => "f32",
                BaseType::F64 => "f64",
                BaseType::I32 => "i32",
                BaseType::I64 => "i64",
                BaseType::U32 => "u32",
                BaseType::U64 => "u64",
                _ => "f64",
            };
            ArgCoercion::CastNumeric(cast.to_string())
        }
    }
}

fn safety_type_from_arg_context(ctx: &ArgContext) -> SafetyType {
    match ctx.arg_expr {
        Expression::Literal { value, .. } => match value {
            Literal::String(_) => SafetyType::borrowed(BaseType::String, Region::fresh(0)),
            Literal::Int(_) | Literal::Float(_) => SafetyType::copy(BaseType::F32),
            Literal::Bool(_) => SafetyType::copy(BaseType::Bool),
            _ => SafetyType::owned(BaseType::Inferred),
        },
        Expression::Identifier { .. } => SafetyType::owned(BaseType::Inferred),
        Expression::FieldAccess { .. } => SafetyType::borrowed(BaseType::Inferred, Region::fresh(1)),
        _ => SafetyType::owned(BaseType::Inferred),
    }
}

/// Compute coercion via shared `ir::coercion` engine (signature-driven).
pub fn compute_arg_coercion_ir(ctx: &ArgContext) -> ArgCoercion {
    let param_idx = ctx.sig.arg_param_index(ctx.arg_index);
    let expected = safety_type_from_signature_param(ctx.sig, param_idx);
    let actual = safety_type_from_arg_context(ctx);
    let kind = compute_coercion(&actual, &expected);
    coercion_kind_to_arg(kind)
}

/// Compute the single correct coercion for a call-site argument.
///
/// This replaces ~38 sequential heuristic phases with one decision based on:
/// 1. What the generated Rust formal parameter expects (type + ownership)
/// 2. What the argument expression provides
/// 3. Copy semantics
pub fn compute_arg_coercion(ctx: &ArgContext) -> ArgCoercion {
    if use_ir_coercion_engine() {
        return compute_arg_coercion_ir(ctx);
    }

    let param_idx = ctx.sig.arg_param_index(ctx.arg_index);
    let effective = effective_param_ownership(ctx.sig, param_idx);

    let formal_type = ctx.sig.formal_param_type(param_idx);
    let converged_type = ctx.sig.param_types.get(param_idx);

    let formal_is_text = formal_type.is_some_and(types::is_windjammer_text_type);
    let formal_is_ref = formal_type
        .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
    let formal_is_str_ref = formal_type.is_some_and(string_utilities::param_is_rust_str_ref);
    let converged_is_str_ref = converged_type
        .is_some_and(string_utilities::param_is_rust_str_ref);
    let callee_expects_str_ref = formal_is_str_ref || converged_is_str_ref;

    // Rule 0: Copy literals (int/float/bool) never need coercion.
    if ctx.is_copy_literal {
        return ArgCoercion::PassThrough;
    }

    // Rule 1: &mut parameters.
    if effective == OwnershipMode::MutBorrowed {
        if ctx.arg_already_ref || ctx.is_copy_literal {
            return ArgCoercion::PassThrough;
        }
        return ArgCoercion::MutBorrow;
    }

    // Rule 2: String literal handling.
    if ctx.is_string_literal {
        return coerce_string_literal(ctx, effective, formal_is_text, callee_expects_str_ref, formal_is_ref);
    }

    // Rule 3: Copy formal types pass by value — strip spurious &, never add &.
    // Exception: collection key lookups (HashMap::get(&k)) always need &.
    if ctx.is_formal_copy && !ctx.is_collection_key {
        if ctx.arg_already_ref {
            return ArgCoercion::StripRef;
        }
        return ArgCoercion::PassThrough;
    }

    // Rule 4: Callee expects &str (either from formal or converged type).
    if callee_expects_str_ref {
        if ctx.arg_already_ref {
            return ArgCoercion::PassThrough;
        }
        // Owned String -> &str: just borrow (Rust auto-derefs &String to &str).
        return ArgCoercion::BorrowString;
    }

    // Rule 5: Callee expects owned String (formal is text, not ref).
    if formal_is_text && !formal_is_ref && effective == OwnershipMode::Owned {
        if ctx.arg_already_ref {
            return ArgCoercion::Clone;
        }
        return ArgCoercion::PassThrough;
    }

    // Rule 6: Effective ownership is Borrowed.
    if effective == OwnershipMode::Borrowed {
        if ctx.arg_already_ref {
            return ArgCoercion::PassThrough;
        }

        // Collection keys always need &.
        if ctx.is_collection_key {
            return ArgCoercion::Borrow;
        }

        // Struct literals don't need borrowing (they're temporary values).
        if matches!(ctx.arg_expr, Expression::StructLiteral { .. }) {
            return ArgCoercion::PassThrough;
        }

        return ArgCoercion::Borrow;
    }

    // Rule 7: Effective ownership is Owned but formal is non-Copy reference type.
    if effective == OwnershipMode::Owned && formal_is_ref {
        if ctx.arg_already_ref {
            return ArgCoercion::PassThrough;
        }
        return ArgCoercion::Borrow;
    }

    // Rule 8: Owned formal, owned effective — pass through.
    ArgCoercion::PassThrough
}

/// Handle string literal coercion separately for clarity.
fn coerce_string_literal(
    ctx: &ArgContext,
    effective: OwnershipMode,
    formal_is_text: bool,
    callee_expects_str_ref: bool,
    formal_is_ref: bool,
) -> ArgCoercion {
    // String literal to &str param: literals coerce to &str automatically in Rust.
    if callee_expects_str_ref {
        return ArgCoercion::PassThrough;
    }

    // String literal to owned String param: need .to_string().
    if formal_is_text && !formal_is_ref && effective == OwnershipMode::Owned {
        return ArgCoercion::ToOwnedString;
    }

    // Borrowed text: string literals coerce to &str in Rust, no conversion needed.
    if effective == OwnershipMode::Borrowed {
        return ArgCoercion::PassThrough;
    }

    // Collection key with string literal: pass through (HashMap auto-borrows string literals).
    if ctx.is_collection_key {
        return ArgCoercion::PassThrough;
    }

    // Default for string literals: Rust treats `"..."` as `&str`, so if callee
    // expects owned, convert; otherwise pass through.
    if effective == OwnershipMode::Owned && formal_is_text {
        return ArgCoercion::ToOwnedString;
    }

    ArgCoercion::PassThrough
}

/// Whether the typed lowering engine is enabled (default: true).
/// Set `WJ_TYPED_LOWERING=0` to fall back to the legacy heuristic pipeline.
pub fn is_typed_lowering_enabled() -> bool {
    std::env::var("WJ_TYPED_LOWERING")
        .map(|v| v != "0")
        .unwrap_or(true)
}

/// Post-hoc correction pass: fix known bad patterns left by the legacy pipeline.
///
/// This runs AFTER the legacy heuristic pipeline has produced an arg_str. It only
/// corrects specific known-bad patterns, leaving correct output untouched. This is
/// the bridge to eventual full replacement of the pipeline.
///
/// Returns true if a correction was applied.
pub fn correct_legacy_output(
    sig: &FunctionSignature,
    arg_index: usize,
    arg_expr: &Expression,
    arg_str: &mut String,
    is_formal_copy: bool,
    is_collection_key: bool,
) -> bool {
    let param_idx = sig.arg_param_index(arg_index);
    let formal_type = sig.formal_param_type(param_idx);
    let effective = effective_param_ownership(sig, param_idx);

    // Correction 1: Strip spurious & on Copy-type formals (except collection keys).
    // The legacy pipeline sometimes adds & via auto-borrow heuristics even when
    // the formal type is a Copy type that should be passed by value.
    if is_formal_copy
        && !is_collection_key
        && arg_str.starts_with('&')
        && !arg_str.starts_with("&mut ")
        && effective == OwnershipMode::Owned
    {
        if formal_type.is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
        }) {
            *arg_str = arg_str[1..].to_string();
            return true;
        }
    }

    // Correction 2: Strip spurious .clone() when callee expects &str.
    // Legacy pipeline sometimes adds .clone() to String args when &str is expected.
    let converged_is_str_ref = sig
        .param_types
        .get(param_idx)
        .is_some_and(string_utilities::param_is_rust_str_ref);
    let formal_is_str_ref = formal_type
        .is_some_and(string_utilities::param_is_rust_str_ref);

    let formal_is_wj_text = formal_type
        .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t));
    if (converged_is_str_ref || formal_is_str_ref || formal_is_wj_text)
        && effective == OwnershipMode::Borrowed
        && arg_str.ends_with(".clone()")
        && !arg_str.starts_with('&')
    {
        *arg_str = arg_str[..arg_str.len() - ".clone()".len()].to_string();
        if !arg_str.starts_with('&') {
            *arg_str = format!("&{}", arg_str);
        }
        return true;
    }

    // Correction 3: Add & when effective ownership is Borrowed but arg doesn't have it.
    // This fixes cross-file calls where the callee's signature specifies Borrowed
    // but the legacy pipeline didn't add & (common with Vec<T> params in metadata).
    // IMPORTANT: Only for constructor expressions (Vec::new(), vec![...]) where we KNOW
    // the argument is owned and the callee expects a reference. Don't touch identifiers
    // or field accesses — they might already be references or handle auto-reborrow.
    if effective == OwnershipMode::Borrowed
        && !arg_str.starts_with('&')
        && !is_collection_key
    {
        let is_string_lit =
            crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr);
        let formal_is_text = formal_type
            .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t));

        // Only add & to constructor calls and struct literals — never to identifiers
        // or field accesses, which may already be references.
        let is_constructor_expr = arg_str.starts_with("Vec::new()")
            || arg_str.starts_with("vec![")
            || arg_str.starts_with("HashMap::new()")
            || arg_str.starts_with("HashSet::new()");

        if is_constructor_expr && !is_string_lit && !formal_is_text && !is_formal_copy {
            *arg_str = format!("&{}", arg_str);
            return true;
        }
    }

    // Correction 4: Add .to_string() when callee expects owned String but arg is &str literal.
    // Only if the legacy pipeline hasn't already converted it (String::from, .to_string(), etc.)
    if effective == OwnershipMode::Owned
        && !arg_str.starts_with('&')
    {
        let is_string_lit =
            crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr);
        let formal_is_text = formal_type
            .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t));
        let already_converted = arg_str.ends_with(".to_string()")
            || arg_str.starts_with("String::from(")
            || arg_str.ends_with(".to_owned()");

        if is_string_lit && formal_is_text && !already_converted {
            *arg_str = format!("{}.to_string()", arg_str);
            return true;
        }
    }

    // Correction 5: Strip spurious & on struct literals when callee takes owned.
    // The legacy pipeline sometimes adds & to struct literals when the formal type is owned.
    if effective == OwnershipMode::Owned
        && arg_str.starts_with("&(")
        && !is_collection_key
    {
        if formal_type.is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
        }) {
            // Strip the leading & — the struct literal is already owned
            *arg_str = arg_str[1..].to_string();
            return true;
        }
    }
    // Also strip &TypeName { ... } patterns
    if effective == OwnershipMode::Owned
        && arg_str.starts_with('&')
        && !arg_str.starts_with("&mut ")
        && !is_collection_key
    {
        if matches!(arg_expr, Expression::StructLiteral { .. }) {
            if formal_type.is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            }) {
                *arg_str = arg_str[1..].to_string();
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::OwnershipMode;
    use crate::parser::{Expression, Literal, Type};

    fn make_sig(
        name: &str,
        param_types: Vec<Type>,
        formal_param_types: Vec<Type>,
        ownership: Vec<OwnershipMode>,
        has_self: bool,
    ) -> FunctionSignature {
        FunctionSignature {
            name: name.into(),
            param_types,
            formal_param_types,
            param_ownership: ownership,
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: has_self,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    fn ident_expr(name: &str) -> Expression<'static> {
        Expression::Identifier {
            name: name.into(),
            location: Default::default(),
        }
    }

    fn string_lit_expr() -> Expression<'static> {
        Expression::Literal {
            value: Literal::String("hello".into()),
            location: Default::default(),
        }
    }

    fn int_lit_expr(val: i64) -> Expression<'static> {
        Expression::Literal {
            value: Literal::Int(val),
            location: Default::default(),
        }
    }

    // --- Class 1: Missing & — owned value where reference expected ---

    #[test]
    fn owned_string_to_borrowed_str_ref_adds_borrow() {
        let sig = make_sig(
            "SceneManager::is_registered",
            vec![Type::Custom("Self".into()), Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![Type::Custom("Self".into()), Type::String],
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            true,
        );
        let arg = ident_expr("name");
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "name.clone()",
            is_formal_copy: false,
            arg_already_ref: false,
            is_string_literal: false,
            is_copy_literal: false,
            is_collection_key: false,
            method_name: "is_registered",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::BorrowString,
            "owned String to &str param should borrow, not clone");
    }

    // --- Class 2: Excessive & — ref where owned expected ---

    #[test]
    fn owned_param_with_ref_arg_passes_through_for_copy() {
        let sig = make_sig(
            "Renderer::upload",
            vec![Type::Custom("Self".into()), Type::Custom("VoxelWorldData".into())],
            vec![Type::Custom("Self".into()), Type::Custom("VoxelWorldData".into())],
            vec![OwnershipMode::MutBorrowed, OwnershipMode::Owned],
            true,
        );
        let arg = ident_expr("world");
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "&world",
            is_formal_copy: true,
            arg_already_ref: true,
            is_string_literal: false,
            is_copy_literal: false,
            is_collection_key: false,
            method_name: "upload",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::StripRef,
            "Copy formal with & arg should strip the &");
    }

    // --- Class 3: String literal missing .to_string() ---

    #[test]
    fn string_literal_to_owned_string_param() {
        let sig = make_sig(
            "Material::set_name",
            vec![Type::Custom("Self".into()), Type::String],
            vec![Type::Custom("Self".into()), Type::String],
            vec![OwnershipMode::MutBorrowed, OwnershipMode::Owned],
            true,
        );
        let arg = string_lit_expr();
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "\"Metal\"",
            is_formal_copy: false,
            arg_already_ref: false,
            is_string_literal: true,
            is_copy_literal: false,
            is_collection_key: false,
            method_name: "set_name",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::ToOwnedString,
            "string literal to owned String param needs .to_string()");
    }

    // --- Class 4: Copy type over-borrowed ---

    #[test]
    fn copy_type_not_borrowed() {
        let sig = make_sig(
            "render_voxels",
            vec![Type::Custom("Camera".into())],
            vec![Type::Custom("Camera".into())],
            vec![OwnershipMode::Owned],
            false,
        );
        let arg = ident_expr("camera");
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "camera",
            is_formal_copy: true,
            arg_already_ref: false,
            is_string_literal: false,
            is_copy_literal: false,
            is_collection_key: false,
            method_name: "render_voxels",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::PassThrough,
            "Copy type with owned formal should pass through");
    }

    // --- Class 5: Inverse string coercion ---

    #[test]
    fn string_literal_to_str_ref_passes_through() {
        let sig = make_sig(
            "contains",
            vec![Type::Custom("Self".into()), Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![Type::Custom("Self".into()), Type::Custom("string".into())],
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            true,
        );
        let arg = string_lit_expr();
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "\"hello\"",
            is_formal_copy: false,
            arg_already_ref: false,
            is_string_literal: true,
            is_copy_literal: false,
            is_collection_key: false,
            method_name: "contains",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::PassThrough,
            "string literal to &str should pass through (Rust auto-coerces)");
    }

    // --- Collection key: HashMap::get needs & ---

    #[test]
    fn collection_key_adds_borrow() {
        let sig = make_sig(
            "HashMap::get",
            vec![Type::Custom("Self".into()), Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![Type::Custom("Self".into()), Type::String],
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            true,
        );
        let arg = ident_expr("key");
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "key",
            is_formal_copy: false,
            arg_already_ref: false,
            is_string_literal: false,
            is_copy_literal: false,
            is_collection_key: true,
            method_name: "get",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::BorrowString,
            "collection key lookup should borrow the string");
    }

    // --- Int literal passes through ---

    #[test]
    fn int_literal_passes_through() {
        let sig = make_sig(
            "Vec::push",
            vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
            vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
            vec![OwnershipMode::MutBorrowed, OwnershipMode::Owned],
            true,
        );
        let arg = int_lit_expr(42);
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "42",
            is_formal_copy: true,
            arg_already_ref: false,
            is_string_literal: false,
            is_copy_literal: true,
            is_collection_key: false,
            method_name: "push",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::PassThrough);
    }

    // --- Borrowed non-copy adds & ---

    #[test]
    fn borrowed_non_copy_adds_borrow() {
        // Free function (not type-qualified) with Vec<String> param that converged to Borrowed
        let sig = make_sig(
            "check_collisions",
            vec![
                Type::Parameterized("Vec".into(), vec![Type::Custom("AABB".into())]),
            ],
            vec![
                Type::Parameterized("Vec".into(), vec![Type::Custom("AABB".into())]),
            ],
            vec![OwnershipMode::Borrowed],
            false,
        );
        let arg = ident_expr("walls");
        let ctx = ArgContext {
            sig: &sig,
            arg_index: 0,
            arg_expr: &arg,
            arg_str: "walls",
            is_formal_copy: false,
            arg_already_ref: false,
            is_string_literal: false,
            is_copy_literal: false,
            is_collection_key: false,
            method_name: "check_collisions",
        };
        let coercion = compute_arg_coercion(&ctx);
        assert_eq!(coercion, ArgCoercion::Borrow,
            "borrowed Vec param should add &");
    }
}
