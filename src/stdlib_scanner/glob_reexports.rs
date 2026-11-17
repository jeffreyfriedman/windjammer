//! A runtime module that re-exports a private child with `pub use child::*;` (`time.rs`: `mod calendar; pub use
//! calendar::*;`) publishes the child's functions under its own path: `time::parse_duration_ms` is the declaration
//! `time::calendar::parse_duration_ms`, and call sites read its formals (`text: &str`) under the path they write.

use super::*;

/// `time::calendar` -> `time` when `src/time.rs` (or `src/time/mod.rs`) declares `pub use calendar::*;`.
fn glob_reexporting_parent(module_name: &str) -> Option<&str> {
    let (parent, child) = module_name.rsplit_once("::")?;
    let reexport = format!("pub use {child}::*;");
    let text = [format!("src/{}.rs", parent.replace("::", "/")), format!("src/{}/mod.rs", parent.replace("::", "/"))]
        .iter()
        .find_map(|p| crate::embedded_assets::text(crate::embedded_assets::Asset::RuntimeCrate, p))?;
    text.lines().any(|l| l.trim() == reexport).then_some(parent)
}

/// Register `sig` (a free function of `module_name`) under the path of the module that glob re-exports it.
pub(super) fn register_glob_reexport(registry: &mut SignatureRegistry, module_name: &str, sig: &FunctionSignature, method_name: &str, mark_sanitizer: bool) {
    let Some(parent) = glob_reexporting_parent(module_name) else { return };
    let mut aliased = sig.clone();
    aliased.name = format!("{parent}::{method_name}");
    if mark_sanitizer {
        registry.register_taint_sanitizer(&aliased.name);
    }
    registry.add_function(aliased.name.clone(), aliased);
}
