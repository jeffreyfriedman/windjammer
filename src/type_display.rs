//! Format Windjammer [`Type`] values for IDE/MCP output.

use crate::parser::ast::types::Type;

/// Render a Windjammer type as a human-readable string (matches parser conventions).
pub fn format_wj_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Int32 => "i32".to_string(),
        Type::Uint => "uint".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Custom(name) | Type::Generic(name) => name.clone(),
        Type::Reference(inner) => format!("&{}", format_wj_type(inner)),
        Type::MutableReference(inner) => format!("&mut {}", format_wj_type(inner)),
        Type::RawPointer { mutable, pointee } => {
            if *mutable {
                format!("*mut {}", format_wj_type(pointee))
            } else {
                format!("*const {}", format_wj_type(pointee))
            }
        }
        Type::Option(inner) => format!("Option<{}>", format_wj_type(inner)),
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            format_wj_type(ok),
            format_wj_type(err)
        ),
        Type::Vec(inner) => format!("Vec<{}>", format_wj_type(inner)),
        Type::Array(inner, size) => format!("[{}; {}]", format_wj_type(inner), size),
        Type::Tuple(types) => {
            let parts: Vec<String> = types.iter().map(format_wj_type).collect();
            format!("({})", parts.join(", "))
        }
        Type::Parameterized(base, args) => {
            let parts: Vec<String> = args.iter().map(format_wj_type).collect();
            format!("{}<{}>", base, parts.join(", "))
        }
        Type::Associated(base, name) => format!("{}::{}", base, name),
        Type::TraitObject(trait_name) => format!("dyn {}", trait_name),
        Type::ImplTrait(trait_name) => format!("trait {}", trait_name),
        Type::Infer => "_".to_string(),
        Type::FunctionPointer {
            params,
            return_type,
        } => {
            let param_strs: Vec<String> = params.iter().map(format_wj_type).collect();
            match return_type {
                Some(ret) => format!("fn({}) -> {}", param_strs.join(", "), format_wj_type(ret)),
                None => format!("fn({})", param_strs.join(", ")),
            }
        }
    }
}
