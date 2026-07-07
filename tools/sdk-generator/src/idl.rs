//! Thin bridge from sdk-generator to compiler FFI IDL (WJ-IMPL-02).

pub use windjammer::ffi::{
    generate_c_header, IdlField, IdlFunction, IdlModule, IdlParam, IdlType, IdlTypeKind,
};
pub use windjammer::metadata::ModuleMetadata;

/// Load IDL from a `.wj.meta` JSON file on disk.
pub fn idl_from_meta_file(path: &std::path::Path) -> anyhow::Result<IdlModule> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
    let meta: ModuleMetadata = serde_json::from_str(&text)?;
    Ok(IdlModule::from_module_metadata(&meta))
}

/// Generate a C header file from a `.wj.meta` path.
pub fn generate_c_header_from_meta(path: &std::path::Path) -> anyhow::Result<String> {
    let idl = idl_from_meta_file(path)?;
    Ok(generate_c_header(&idl))
}
