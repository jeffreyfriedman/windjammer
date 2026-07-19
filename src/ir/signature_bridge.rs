//! Bridge analyzer signatures to IR `SafetyType` values.

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::ir::node::parser_type_to_base_type;
use crate::ir::safety_type::{BaseType, EffectSet, OwnedType, Region, SafetyType, TaintStatus, ConstEval};
use crate::parser::Type;

/// Build a `SafetyType` from a parser `Type`, extracting ownership from reference
/// wrappers when present and falling back to analyzer ownership mode otherwise.
pub fn safety_type_from_parser_type(ty: &Type, fallback_mode: Option<OwnershipMode>) -> SafetyType {
    let (base, ownership) = match ty {
        Type::Reference(inner) => (
            parser_type_to_base_type(inner),
            OwnedType::Ref(Region::fresh(0)),
        ),
        Type::MutableReference(inner) => (
            parser_type_to_base_type(inner),
            OwnedType::MutRef(Region::fresh(1)),
        ),
        other => {
            let base = parser_type_to_base_type(other);
            let ownership = fallback_mode
                .map(ownership_mode_to_owned)
                .unwrap_or(OwnedType::Owned);
            (base, ownership)
        }
    };

    SafetyType {
        base,
        ownership,
        effects: EffectSet::pure(),
        taint: TaintStatus::Clean,
        const_eval: ConstEval::Runtime,
        exec_mode: None,
    }
}

/// Expected safety type for a callee parameter at a call site.
///
/// Prefer converged `param_types` (includes Phase-3 Reference wrap and str_ref `&str`)
/// over bare `formal_param_types` so call-site coercion matches emitted Rust signatures.
///
/// When the registry marks a plain `string` param as `Owned`, that contract wins over a
/// stale `Reference(str)` wrap left from body-inferred borrow analysis.
pub fn safety_type_from_signature_param(sig: &FunctionSignature, param_idx: usize) -> SafetyType {
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::MutBorrowed)
    ) {
        if let Some(formal) = sig.formal_param_type(param_idx) {
            return safety_type_from_parser_type(
                &Type::MutableReference(Box::new(formal.clone())),
                Some(OwnershipMode::MutBorrowed),
            );
        }
    }

    // Registry-emitted `&str` contract beats stale owned WJ `string` formals.
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx) {
        return SafetyType {
            base: BaseType::String,
            ownership: OwnedType::Ref(Region::fresh(3)),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: crate::ir::safety_type::ConstEval::Runtime,
            exec_mode: None,
        };
    }

    // Codegen refresh recorded owned Rust formals — beats stale body-converged Reference(T).
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx) {
        if let Some(formal) = sig.formal_param_type(param_idx) {
            return safety_type_from_parser_type(formal, Some(OwnershipMode::Owned));
        }
    }

    // WJ bare non-Copy struct formals (`key: Key`) are owned API contracts unless the
    // converged registry or emitted Rust formals record a borrow, or codegen marked
    // an explicit owned-emission contract (forward-ref / mixed-forwarder callers).
    if let Some(formal) = sig.formal_param_type(param_idx) {
        let wj_owned_struct = !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !is_plain_windjammer_string_type(formal)
            && !crate::codegen::rust::type_analysis::is_copy_type(formal);
        if wj_owned_struct {
            let emits_ref = sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                .unwrap_or(false);
            let converged_ref = sig
                .param_types
                .get(param_idx)
                .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
            let effective = crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                sig, param_idx,
            );
            let converged_borrow = converged_ref
                || matches!(
                    effective,
                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                );
            if !emits_ref && !converged_borrow {
                return safety_type_from_parser_type(formal, Some(OwnershipMode::Owned));
            }
        }
    }

    // Converged Rust param types (Reference/MutableReference) are the emitted contract.
    if let Some(ty) = sig.param_types.get(param_idx) {
        if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
            && crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
        {
            return SafetyType {
                base: BaseType::String,
                ownership: OwnedType::Ref(Region::fresh(3)),
                effects: EffectSet::pure(),
                taint: TaintStatus::Clean,
                const_eval: ConstEval::Runtime,
                exec_mode: None,
            };
        }
        if matches!(ty, Type::Reference(_) | Type::MutableReference(_))
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, param_idx,
            )
        {
            return safety_type_from_parser_type(ty, None);
        }
    }

    if crate::codegen::rust::call_signature_resolution::static_impl_text_borrows_at_call_site(
        sig, param_idx,
    ) {
        return SafetyType {
            base: BaseType::String,
            ownership: OwnedType::Ref(Region::fresh(4)),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }

    let effective = crate::codegen::rust::call_signature_resolution::effective_param_ownership(
        sig,
        param_idx,
    );
    let mode = Some(effective);
    if matches!(effective, OwnershipMode::Owned) {
        let is_plain_string = sig
            .formal_param_type(param_idx)
            .is_some_and(is_plain_windjammer_string_type)
            || sig
                .param_types
                .get(param_idx)
                .is_some_and(is_plain_windjammer_string_type);
        if is_plain_string {
            return SafetyType {
                base: BaseType::String,
                ownership: OwnedType::Owned,
                effects: EffectSet::pure(),
                taint: TaintStatus::Clean,
                const_eval: ConstEval::Runtime,
                exec_mode: None,
            };
        }
    }

    if let Some(ty) = sig.param_types.get(param_idx) {
        return safety_type_from_parser_type(ty, mode);
    }
    if let Some(ty) = sig.formal_param_type(param_idx) {
        return safety_type_from_parser_type(ty, mode);
    }
    SafetyType {
        base: BaseType::Inferred,
        ownership: OwnedType::Owned,
        effects: EffectSet::pure(),
        taint: TaintStatus::Clean,
        const_eval: ConstEval::Runtime,
        exec_mode: None,
    }
}

pub fn ownership_mode_to_owned(mode: OwnershipMode) -> OwnedType {
    match mode {
        OwnershipMode::Owned => OwnedType::Owned,
        OwnershipMode::Borrowed => OwnedType::Ref(Region::fresh(0)),
        OwnershipMode::MutBorrowed => OwnedType::MutRef(Region::fresh(1)),
    }
}

/// Infer the actual `SafetyType` of a call argument from emitted target text.
///
/// Shared by Go/JavaScript backends that lack full Rust `CodeGenerator` context.
/// Solver-resolved types from [`resolve_call_arg_actual_type`] refine this when available.
pub fn safety_type_from_emit_text(arg_str: &str) -> SafetyType {
    if arg_str.ends_with(".clone()") {
        return SafetyType::owned(BaseType::Inferred);
    }
    if arg_str.starts_with("&mut ") {
        return SafetyType {
            base: BaseType::Inferred,
            ownership: OwnedType::MutRef(Region::fresh(1)),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }
    if arg_str.starts_with('&') {
        return SafetyType::borrowed(BaseType::Inferred, Region::fresh(0));
    }
    let trimmed = arg_str.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return SafetyType::borrowed(BaseType::String, Region::fresh(2));
    }
    SafetyType::owned(BaseType::Inferred)
}

/// Lookup solver-resolved safety type for a binding across all IR functions in a module.
pub fn safety_type_from_ir_binding(
    module: &crate::ir::pipeline::IrModule,
    binding: &str,
) -> Option<SafetyType> {
    for ir_fn in &module.functions {
        if let Some(st) = ir_fn.param_types.get(binding) {
            return Some(st.clone());
        }
        if let Some(st) = ir_fn.local_types.get(binding) {
            return Some(st.clone());
        }
    }
    None
}

/// Resolve actual call-site `SafetyType`: emit-text shape + solver binding types when present.
pub fn resolve_call_arg_actual_type(
    module: &crate::ir::pipeline::IrModule,
    arg_str: &str,
) -> SafetyType {
    let from_emit = safety_type_from_emit_text(arg_str);
    let Some(binding) = simple_binding_from_emit_text(arg_str) else {
        return from_emit;
    };
    let Some(from_ir) = safety_type_from_ir_binding(module, &binding) else {
        return from_emit;
    };
    merge_actual_safety_types(from_emit, from_ir)
}

fn simple_binding_from_emit_text(arg_str: &str) -> Option<String> {
    let mut s = arg_str.trim();
    if let Some(base) = s.strip_suffix(".clone()") {
        s = base;
    }
    if let Some(base) = s.strip_prefix("&mut ") {
        s = base;
    } else if let Some(base) = s.strip_prefix('&') {
        s = base;
    }
    s = s.trim();
    if s.is_empty() || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some(s.to_string())
}

fn merge_actual_safety_types(emit: SafetyType, ir: SafetyType) -> SafetyType {
    SafetyType {
        base: if ir.base != BaseType::Inferred {
            ir.base
        } else {
            emit.base
        },
        ownership: if !matches!(emit.ownership, OwnedType::Owned) {
            emit.ownership
        } else if ir.ownership != OwnedType::Inferred {
            ir.ownership
        } else {
            emit.ownership
        },
        effects: emit.effects,
        taint: emit.taint,
        const_eval: emit.const_eval,
        exec_mode: emit.exec_mode,
    }
}

/// Map solver-resolved IR ownership back to analyzer `OwnershipMode` for the signature registry.
pub fn owned_type_to_ownership_mode(ownership: &OwnedType) -> OwnershipMode {
    match ownership {
        OwnedType::Owned | OwnedType::Copy => OwnershipMode::Owned,
        OwnedType::Ref(_) => OwnershipMode::Borrowed,
        OwnedType::MutRef(_) => OwnershipMode::MutBorrowed,
        OwnedType::Inferred => OwnershipMode::Owned,
    }
}

/// Write solver-resolved parameter ownership from IR back into the converged registry.
/// Enables cross-file IR lowering and delegation wrappers to see prior modules' contracts.
pub fn sync_ir_ownership_to_registry(
    ir_functions: &[crate::ir::node::IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
    registry: &mut crate::analyzer::SignatureRegistry,
) {
    use crate::analyzer::OwnershipMode;

    for (ir_fn, af) in ir_functions.iter().zip(analyzed.iter()) {
        let mut lookup_keys = vec![ir_fn.name.clone()];
        if !lookup_keys.contains(&af.decl.name) {
            lookup_keys.push(af.decl.name.clone());
        }
        let Some(base_key) = lookup_keys
            .iter()
            .find(|k| registry.get_signature(k).is_some())
            .cloned()
        else {
            continue;
        };
        let Some(mut sig) = registry.get_signature(&base_key).cloned() else {
            continue;
        };
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            let Some(st) = ir_fn.param_types.get(&param.name) else {
                continue;
            };
            let mode = owned_type_to_ownership_mode(&st.ownership);
            if idx >= sig.param_ownership.len() {
                continue;
            }
            sig.param_ownership[idx] = mode;
            if matches!(mode, OwnershipMode::Owned)
                && idx < sig.param_types.len()
                && matches!(
                    sig.param_types[idx],
                    Type::Reference(_) | Type::MutableReference(_)
                )
            {
                if let Type::Reference(inner) | Type::MutableReference(inner) =
                    sig.param_types[idx].clone()
                {
                    sig.param_types[idx] = *inner;
                    if idx < sig.formal_param_types.len() {
                        sig.formal_param_types[idx] = sig.param_types[idx].clone();
                    }
                }
            }
        }
        registry.add_function(base_key, sig);
    }
}

fn is_plain_windjammer_string_type(ty: &Type) -> bool {
    match ty {
        Type::String => true,
        Type::Custom(name) => name == "string",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::FunctionSignature;

    fn sig_with_types(param_types: Vec<Type>, ownership: Vec<OwnershipMode>) -> FunctionSignature {
        FunctionSignature {
            name: "test_fn".into(),
            formal_param_types: param_types.clone(),
            param_types,
            param_ownership: ownership,
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn call_site_expected_prefers_converged_reference_wrap() {
        let sig = sig_with_types(
            vec![Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![OwnershipMode::Borrowed],
        );
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::Ref(_)));
        assert!(matches!(expected.base, BaseType::Custom(ref n) if n == "str"));
    }

    #[test]
    fn module_owned_string_stays_owned() {
        let sig = sig_with_types(vec![Type::String], vec![OwnershipMode::Owned]);
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::Owned));
        assert!(matches!(expected.base, BaseType::String));
    }

    #[test]
    fn emit_text_borrow_and_binding_lookup() {
        let borrowed = safety_type_from_emit_text("&key");
        assert!(matches!(borrowed.ownership, OwnedType::Ref(_)));

        let owned = safety_type_from_emit_text("key.clone()");
        assert!(matches!(owned.ownership, OwnedType::Owned));
    }
}
