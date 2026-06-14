//! Per-file declaration stub and type registry collection for multipass library builds.
//!
//! Scans each parsed `.wj` file for float/int inference signatures, struct field
//! layouts, and ownership declaration stubs before the global convergence passes.
//! Safe to run in parallel; merge preserves source-file order for determinism.

use crate::analyzer::SignatureRegistry;
use crate::metadata::CrateMetadata;
use crate::parser::ast::core::{Item, Program};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Registry data extracted from one source file (parallel-safe).
#[derive(Debug, Clone)]
pub struct DeclarationStubContribution {
    pub float_signatures: HashMap<
        String,
        (
            Vec<crate::parser::ast::types::Type>,
            Option<crate::parser::ast::types::Type>,
        ),
    >,
    pub struct_fields: HashMap<String, HashMap<String, crate::parser::ast::types::Type>>,
    pub struct_defining_module_paths: HashMap<String, Vec<Vec<String>>>,
    pub stub_registry: SignatureRegistry,
    pub file_stem: String,
    pub module_path: String,
}

pub fn merge_struct_fields_from_items(
    items: &[Item<'_>],
    module_prefix: &[String],
    global_struct_fields: &mut HashMap<String, HashMap<String, crate::parser::ast::types::Type>>,
    struct_defining_module_paths: &mut HashMap<String, Vec<Vec<String>>>,
) {
    use crate::type_inference::struct_field_registry;
    for item in items {
        match item {
            Item::Struct { decl, .. } => {
                let qualified =
                    struct_field_registry::qualify_struct_key(module_prefix, &decl.name);
                let mut fields = HashMap::new();
                for field in &decl.fields {
                    fields.insert(field.name.clone(), field.field_type.clone());
                }
                global_struct_fields.insert(qualified, fields);
                struct_defining_module_paths
                    .entry(decl.name.clone())
                    .or_default()
                    .push(module_prefix.to_vec());
            }
            Item::Mod { name, items, .. } => {
                let mut next = module_prefix.to_vec();
                next.push(name.clone());
                merge_struct_fields_from_items(
                    items,
                    &next,
                    global_struct_fields,
                    struct_defining_module_paths,
                );
            }
            _ => {}
        }
    }
}

fn collect_float_signatures_from_program(
    program: &Program<'_>,
) -> HashMap<
    String,
    (
        Vec<crate::parser::ast::types::Type>,
        Option<crate::parser::ast::types::Type>,
    ),
> {
    let mut float_signatures = HashMap::new();
    for item in &program.items {
        match item {
            Item::Function { decl, .. } => {
                let param_types: Vec<crate::parser::ast::types::Type> =
                    decl.parameters.iter().map(|p| p.type_.clone()).collect();
                float_signatures.insert(decl.name.clone(), (param_types, decl.return_type.clone()));
            }
            Item::Impl { block, .. } => {
                let base_type_name = block
                    .type_name
                    .split('<')
                    .next()
                    .unwrap_or(&block.type_name);
                for func_decl in &block.functions {
                    let param_types: Vec<crate::parser::ast::types::Type> = func_decl
                        .parameters
                        .iter()
                        .map(|p| p.type_.clone())
                        .collect();
                    let full_name = format!("{}::{}", base_type_name, func_decl.name);
                    float_signatures
                        .insert(full_name, (param_types, func_decl.return_type.clone()));
                }
            }
            _ => {}
        }
    }
    float_signatures
}

/// Scan one parsed file for declaration stubs and type registry entries.
pub fn collect_per_file_declaration_stubs(
    src_base: &Path,
    file: &Path,
    program: &Program<'_>,
) -> DeclarationStubContribution {
    let mut struct_fields = HashMap::new();
    let mut struct_defining_module_paths = HashMap::new();
    let file_module =
        crate::analyzer::type_collector::wj_file_to_module_path(src_base, file).unwrap_or_default();
    merge_struct_fields_from_items(
        &program.items,
        &file_module,
        &mut struct_fields,
        &mut struct_defining_module_paths,
    );

    let stub_registry = SignatureRegistry::from_program_declarations(program);
    let file_stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let module_path = file_module.join("::");

    DeclarationStubContribution {
        float_signatures: collect_float_signatures_from_program(program),
        struct_fields,
        struct_defining_module_paths,
        stub_registry,
        file_stem,
        module_path,
    }
}

/// Merge per-file contributions in source order (deterministic last-wins for float sig keys).
pub fn merge_declaration_stub_contributions(
    global_registry: &mut SignatureRegistry,
    global_float_signatures: &mut HashMap<
        String,
        (
            Vec<crate::parser::ast::types::Type>,
            Option<crate::parser::ast::types::Type>,
        ),
    >,
    global_struct_fields: &mut HashMap<String, HashMap<String, crate::parser::ast::types::Type>>,
    struct_defining_module_paths: &mut HashMap<String, Vec<Vec<String>>>,
    ordered: &[(PathBuf, DeclarationStubContribution)],
    crate_metadata: &mut CrateMetadata,
    parsed_programs: &[Program<'_>],
    source_indices: &[usize],
) {
    for (merge_idx, (file, contrib)) in ordered.iter().enumerate() {
        let program = &parsed_programs[source_indices[merge_idx]];
        crate::metadata::merge_file_skeleton_into_crate(crate_metadata, file, program);

        global_float_signatures.extend(contrib.float_signatures.clone());
        for (k, v) in &contrib.struct_fields {
            global_struct_fields.insert(k.clone(), v.clone());
        }
        for (name, paths) in &contrib.struct_defining_module_paths {
            struct_defining_module_paths
                .entry(name.clone())
                .or_default()
                .extend(paths.clone());
        }

        global_registry.merge(&contrib.stub_registry);
        global_registry.register_module_aliases(
            &contrib.stub_registry,
            &contrib.file_stem,
            &contrib.module_path,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(source: &str, name: &str) -> Program<'static> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(tokens, name.to_string(), source.to_string());
        parser.parse().expect("parse")
    }

    #[test]
    fn test_collect_nested_module_struct_fields() {
        let src = std::path::PathBuf::from("/proj/src");
        let file = src.join("game.wj");
        let program = parse(
            r#"
pub mod inner {
    pub struct Config {
        scale: f32,
    }
}
"#,
            "game.wj",
        );
        let contrib = collect_per_file_declaration_stubs(&src, &file, &program);
        assert!(
            contrib.struct_fields.contains_key("game::inner::Config"),
            "expected qualified struct key, got {:?}",
            contrib.struct_fields.keys().collect::<Vec<_>>()
        );
        let fields = contrib.struct_fields.get("game::inner::Config").unwrap();
        assert!(fields.contains_key("scale"));
    }

    #[test]
    fn test_collect_impl_method_float_signature() {
        let src = std::path::PathBuf::from("/proj/src");
        let file = src.join("map.wj");
        let program = parse(
            r#"
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn bump(self, delta: i32) -> i32 {
        self.value + delta
    }
}
"#,
            "map.wj",
        );
        let contrib = collect_per_file_declaration_stubs(&src, &file, &program);
        assert!(
            contrib.float_signatures.contains_key("Counter::bump"),
            "expected impl method float sig, got {:?}",
            contrib.float_signatures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_merge_order_preserves_last_wins_for_float_sigs() {
        let src = std::path::PathBuf::from("/proj/src");
        let a_file = src.join("a.wj");
        let b_file = src.join("b.wj");
        let prog_a = parse(r#"pub fn foo(x: f32) -> f32 { x }"#, "a.wj");
        let prog_b = parse(r#"pub fn foo(x: i32) -> i32 { x }"#, "b.wj");
        let ca = collect_per_file_declaration_stubs(&src, &a_file, &prog_a);
        let cb = collect_per_file_declaration_stubs(&src, &b_file, &prog_b);

        let mut registry = SignatureRegistry::new();
        let mut float_sigs = HashMap::new();
        let mut struct_fields = HashMap::new();
        let mut struct_paths = HashMap::new();
        let mut crate_meta = CrateMetadata::default();

        merge_declaration_stub_contributions(
            &mut registry,
            &mut float_sigs,
            &mut struct_fields,
            &mut struct_paths,
            &[(a_file.clone(), ca), (b_file.clone(), cb)],
            &mut crate_meta,
            &[prog_a, prog_b],
            &[0, 1],
        );

        let (_, ret) = float_sigs.get("foo").expect("foo sig");
        assert!(
            matches!(ret, Some(t) if format!("{:?}", t).contains("i32")),
            "second file should win for duplicate foo name"
        );
    }
}
