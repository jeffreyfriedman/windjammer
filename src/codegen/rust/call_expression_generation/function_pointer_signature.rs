//! Signature of a call through a function-pointer parameter (`has_item(a, b)`), built from the parameter's type.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::CodeGenerator;

/// When `func_name` is a function-pointer parameter of the current function, its signature (no registry lookup).
pub(super) fn function_pointer_param_signature<'ast>(
    gen: &CodeGenerator<'ast>,
    func_name: &str,
) -> Option<crate::analyzer::FunctionSignature> {
    gen
        .current_function_params
        .iter()
        .find(|p| p.name == func_name)
        .and_then(|param| {
            if let Type::FunctionPointer {
                params,
                return_type,
            } = &param.type_
            {
                let param_ownership: Vec<OwnershipMode> = params
                    .iter()
                    .map(|ty| match ty {
                        Type::String => OwnershipMode::Borrowed,
                        Type::Custom(name) if name == "string" => OwnershipMode::Borrowed,
                        Type::Reference(_) | Type::MutableReference(_) => OwnershipMode::Borrowed,
                        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => {
                            OwnershipMode::Owned
                        }
                        Type::Custom(name)
                            if matches!(
                                name.as_str(),
                                "i32"
                                    | "i64"
                                    | "u32"
                                    | "u64"
                                    | "f32"
                                    | "f64"
                                    | "bool"
                                    | "char"
                                    | "usize"
                                    | "isize"
                            ) =>
                        {
                            OwnershipMode::Owned
                        }
                        _ => OwnershipMode::Owned,
                    })
                    .collect();
                // Mirror Rust `fn(&str, …)` emission so call sites suppress field clones.
                let emitted_rust_ref_params = Some(
                    param_ownership
                        .iter()
                        .map(|o| matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed))
                        .collect(),
                );

                Some(crate::analyzer::FunctionSignature {
                    name: func_name.to_string(),
                    param_types: params.clone(),
                    formal_param_types: params.clone(),
                    param_ownership,
                    return_type: return_type.as_ref().map(|t| (**t).clone()),
                    return_ownership: OwnershipMode::Owned,
                    has_self_receiver: false,
                    is_extern: false,
                    emitted_rust_ref_params,
                    string_ref_string_formal_params: None,
                    field_extract_params: None,
                    forwarding_borrow_params: None,
                })
            } else {
                None
            }
        })
}
