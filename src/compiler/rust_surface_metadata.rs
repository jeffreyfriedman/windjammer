//! Rust crate signature surfaces (ADR-0026) as crate metadata for the build.
//!
//! A surface (`rust/<crate>.wji`) is turned into the `metadata.json` format path dependencies publish, written
//! under `<project>/.wj-cache/rust-surfaces/<crate>/`, and added to the build's external-metadata paths. Every
//! build mode (single file, multipass library, module compiler) already reads those paths, so the ownership
//! pipeline sees the crate's functions under `crate::fn` and call sites borrow what the crate borrows.
//!
//! Passing convention: `string` parameters are borrowed (Rust `&str`); everything else as declared.

use crate::metadata::{CrateMetadata, FunctionSignature, ModuleMetadata};
use crate::package::rust_surface::{self, Surface};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Directory (under the project's `.wj-cache/`) holding the generated metadata.
const CACHE_SUBDIR: &str = "rust-surfaces";

fn is_text(t: &crate::parser::Type) -> bool {
    crate::codegen::rust::types::is_windjammer_text_type(t)
}

/// The metadata that describes `surface` to the ownership pipeline.
pub(crate) fn surface_metadata(surface: &Surface) -> CrateMetadata {
    let mut meta = CrateMetadata::new();
    for f in &surface.fns {
        let types: Vec<&crate::parser::Type> = f.parameters.iter().filter(|p| p.name != "self").map(|p| &p.type_).collect();
        let sig = FunctionSignature {
            params: types.iter().map(|t| ModuleMetadata::serialize_type(t)).collect(),
            formal_params: Vec::new(),
            return_type: f.return_type.as_ref().map(ModuleMetadata::serialize_type),
            is_associated: false,
            parent_type: None,
            param_ownership: types.iter().map(|t| if is_text(t) { "Borrowed" } else { "Owned" }.to_string()).collect(),
            emitted_rust_ref_params: Some(types.iter().map(|t| is_text(t)).collect()),
            string_ref_string_formal_params: None,
            forwarding_borrow_params: None,
            has_self_receiver: false,
            is_extern: false,
            is_trait_method: false,
        };
        meta.functions.insert(f.name.clone(), sig);
    }
    meta
}

/// Write the metadata of every surface of the project that contains `build_path` and return
/// `crate -> metadata dir`. No manifest or no surfaces: empty. Unchanged files are not rewritten.
pub(crate) fn surface_metadata_paths(build_path: &Path) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let Some(root) = crate::module_system::project::find_manifest_dir(build_path) else { return out };
    let crates = rust_surface::load(&root);
    for s in &crates.surfaces {
        let dir = root.join(".wj-cache").join(CACHE_SUBDIR).join(&s.crate_name);
        let Ok(json) = serde_json::to_string_pretty(&surface_metadata(s)) else { continue };
        let file = dir.join("metadata.json");
        if std::fs::read_to_string(&file).ok().as_deref() != Some(json.as_str())
            && (std::fs::create_dir_all(&dir).is_err() || std::fs::write(&file, &json).is_err())
        {
            continue;
        }
        out.insert(s.crate_name.clone(), dir);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_borrowed_and_types_round_trip() {
        let s = rust_surface::parse_surface(
            "demo",
            PathBuf::from("rust/demo.wji"),
            "pub fn f(a: string, n: usize, x: f64) -> usize\npub fn g()\n".to_string(),
        );
        let m = surface_metadata(&s);
        let f = &m.functions["f"];
        assert_eq!(f.param_ownership, ["Borrowed", "Owned", "Owned"]);
        assert_eq!(f.emitted_rust_ref_params, Some(vec![true, false, false]));
        let back = crate::metadata::try_analyzer_signature_from_metadata("f", f).expect("metadata must load");
        assert_eq!(back.param_types.len(), 3);
        assert_eq!(back.return_type, Some(crate::parser::Type::Custom("usize".into())));
        assert!(m.functions["g"].params.is_empty() && m.functions["g"].return_type.is_none());
    }
}
