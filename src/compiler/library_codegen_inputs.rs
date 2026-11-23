//! Crate-wide inputs the library multipass hands each file's code generator.

use crate::analyzer::SignatureRegistry;
use crate::codegen::rust::CodeGenerator;
use crate::CompilationTarget;
use std::collections::HashSet;

/// Stdlib field/enum shapes + FQ Rust paths (`HttpMethod` -> `windjammer_runtime::http::HttpMethod`) so string -> unit-enum
/// coercion works even when the cargo-test binary's exe-relative std lookup fails mid-session, and so sibling imports
/// alone resolve without E0433.
pub(super) fn apply_stdlib_api_types(
    codegen: &mut CodeGenerator<'_>,
    needed_stdlib_modules: &HashSet<String>,
    config: &crate::compiler_config::CompilerConfig,
) {
    if needed_stdlib_modules.is_empty() {
        return;
    }
    let (std_fields, std_variants, std_paths) = super::library_copy_registry::collect_stdlib_api_types_for_modules(
        needed_stdlib_modules,
        config.stdlib_path.as_deref(),
    );
    if !std_fields.is_empty() {
        codegen.set_global_struct_field_types(std_fields);
    }
    if !std_variants.is_empty() {
        codegen.set_global_enum_variant_types(std_variants);
    }
    if !std_paths.is_empty() {
        codegen.set_stdlib_type_rust_paths(std_paths);
    }
}

/// `PartialEq` of a struct whose fields are declared in another module of this crate: decided once over every module's
/// declarations (a per-file pass cannot see them). `registries` is (Copy, explicit Copy, non-Copy) types.
pub(super) fn crate_partial_eq_types(
    programs: &[crate::parser::Program<'_>],
    registries: (&HashSet<String>, &HashSet<String>, &HashSet<String>),
    target: CompilationTarget,
) -> HashSet<String> {
    let mut g = CodeGenerator::new_for_module(SignatureRegistry::new(), target);
    g.set_copy_types_registry(registries.0.clone());
    g.set_explicit_copy_types_registry(registries.1.clone());
    g.set_non_copy_types_registry(registries.2.clone());
    g.collect_partial_eq_types_all(&programs.iter().collect::<Vec<_>>());
    g.partial_eq_types_snapshot()
}
