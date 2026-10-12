//! Return types that name a generic runtime struct (`arena::Handle<T>`): the scanner keeps only the base name,
//! which is not a valid Rust type on its own, so such returns become `Type::Infer`.

use super::parse_function_signature;
use crate::analyzer::FunctionSignature;
use crate::parser::Type;

/// `pub struct Name<..>` declared in a runtime file (e.g. `arena::Handle<T>`).
pub(super) fn generic_struct_names(content: &str) -> std::collections::HashSet<String> {
    content
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub struct "))
        .filter_map(|r| r.split_once('<').map(|(n, _)| n.trim().to_string()))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect()
}

/// The scanner keeps only the base name of a return type (`Handle<T>` -> `Handle`). For a generic struct of the
/// same file that bare name is not a valid Rust type, so let rustc infer it instead (`Vec<_>`, no annotation).
pub(super) fn erase_generic_struct_return(
    sig: &mut FunctionSignature,
    generic_structs: &std::collections::HashSet<String>,
) {
    if let Some(Type::Custom(n)) = &sig.return_type {
        if generic_structs.contains(n) {
            sig.return_type = Some(Type::Infer);
        }
    }
}

/// `parse_function_signature` plus the generic-struct return erasure for the file `content`.
pub(super) fn parse_sig(line: &str, module: &str, content: &str) -> Option<FunctionSignature> {
    let mut sig = parse_function_signature(line, module)?;
    if matches!(&sig.return_type, Some(Type::Custom(_))) {
        erase_generic_struct_return(&mut sig, &generic_struct_names(content));
    }
    Some(sig)
}
