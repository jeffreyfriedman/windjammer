//! Function signature metadata and merging into the analyzer registry.

use crate::analyzer::{FunctionSignature as AnalyzerFunctionSignature, OwnershipMode};
use serde::{Deserialize, Serialize};

use super::ModuleMetadata;

/// Function signature for type inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Parameter types
    pub params: Vec<String>, // Serialized Type (JSON is easier than bincode for now)

    /// Return type (None for unit)
    pub return_type: Option<String>,

    /// Is this an associated function (Type::method)?
    pub is_associated: bool,

    /// Parent type for associated functions (e.g., "Vec3" for Vec3::new)
    pub parent_type: Option<String>,

    /// Inferred ownership modes for each parameter (Owned, Borrowed, MutBorrowed), Debug string form
    #[serde(default)]
    pub param_ownership: Vec<String>,

    /// True when the signature includes a `self` receiver (matches analyzer `has_self_receiver`)
    #[serde(default)]
    pub has_self_receiver: bool,

    /// True when this is an `extern fn` (FFI). Used by codegen to wrap calls in `unsafe`.
    #[serde(default)]
    pub is_extern: bool,
}

/// Build metadata JSON signature from an analyzer registry entry (post-inference).
pub fn metadata_function_sig_from_analyzer(
    sig: &AnalyzerFunctionSignature,
    is_associated: bool,
    parent_type: Option<String>,
) -> FunctionSignature {
    FunctionSignature {
        params: sig
            .param_types
            .iter()
            .map(ModuleMetadata::serialize_type)
            .collect(),
        return_type: sig.return_type.as_ref().map(ModuleMetadata::serialize_type),
        is_associated,
        parent_type,
        param_ownership: sig
            .param_ownership
            .iter()
            .map(|o| format!("{:?}", o))
            .collect(),
        has_self_receiver: sig.has_self_receiver,
        is_extern: sig.is_extern,
    }
}

fn ownership_mode_from_meta_str(s: &str) -> Option<OwnershipMode> {
    match s {
        "Owned" => Some(OwnershipMode::Owned),
        "Borrowed" => Some(OwnershipMode::Borrowed),
        "MutBorrowed" => Some(OwnershipMode::MutBorrowed),
        _ => None,
    }
}

/// Convert loaded metadata to an analyzer signature when `param_ownership` is present (new format).
pub fn try_analyzer_signature_from_metadata(
    name: &str,
    meta_sig: &FunctionSignature,
) -> Option<AnalyzerFunctionSignature> {
    if meta_sig.param_ownership.is_empty() {
        return None;
    }
    let mut param_types = Vec::with_capacity(meta_sig.params.len());
    for p in &meta_sig.params {
        let ty = ModuleMetadata::deserialize_type(p)?;
        param_types.push(ty);
    }
    if param_types.len() != meta_sig.param_ownership.len() {
        return None;
    }
    let mut param_ownership = Vec::with_capacity(meta_sig.param_ownership.len());
    for o in &meta_sig.param_ownership {
        param_ownership.push(ownership_mode_from_meta_str(o)?);
    }
    let return_type = meta_sig
        .return_type
        .as_ref()
        .and_then(|s| ModuleMetadata::deserialize_type(s));

    Some(AnalyzerFunctionSignature {
        name: name.to_string(),
        param_types,
        param_ownership,
        return_type,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: meta_sig.has_self_receiver,
        is_extern: meta_sig.is_extern,
    })
}

pub(in crate::metadata) fn merge_module_metadata_signatures(
    meta: &ModuleMetadata,
    registry: &mut crate::analyzer::SignatureRegistry,
) {
    for (name, sig) in &meta.functions {
        if let Some(a_sig) = try_analyzer_signature_from_metadata(name, sig) {
            registry.add_function(name.clone(), a_sig.clone());
            if !meta.module_path.is_empty() && !name.contains("::") {
                let qualified = format!("{}::{}", meta.module_path, name);
                registry.add_function(qualified, a_sig);
            }
        }
    }
}
