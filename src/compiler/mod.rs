//! Compiler module — builds Windjammer projects for the CLI and integration tests.
//!
//! Submodules split orchestration (`compilation_pipeline`), filesystem and dep metadata
//! (`dependency_resolution`), incremental output handling (`cache_management`), Copy registry
//! (`library_copy_registry`), and the large multipass library path (`library_multipass`).

mod cache_management;
mod compilation_pipeline;
mod dependency_resolution;
mod library_copy_registry;
mod library_multipass;

pub use cache_management::write_if_changed;
pub use compilation_pipeline::{build_project, build_project_ext};

use crate::parser::ast::core::Item;

/// Detect whether a parsed program is a GPU shader file (@vertex, @fragment, @compute).
pub fn is_shader_file(program: &crate::parser::Program) -> bool {
    let registry = crate::decorator_registry::DecoratorRegistry::new();
    for item in &program.items {
        if let Item::Function { decl, .. } = item {
            for decorator in &decl.decorators {
                if registry.is_gpu_decorator(&decorator.name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Remove `Item::Mod` entries whose names are in `filtered_modules` before codegen.
pub fn strip_filtered_mod_items<'ast>(
    items: Vec<crate::parser::ast::core::Item<'ast>>,
    filtered_modules: &std::collections::HashSet<String>,
) -> Vec<crate::parser::ast::core::Item<'ast>> {
    if filtered_modules.is_empty() {
        return items;
    }
    items
        .into_iter()
        .filter(|item| {
            if let crate::parser::ast::core::Item::Mod { name, .. } = item {
                !filtered_modules.contains(name)
            } else {
                true
            }
        })
        .collect()
}
