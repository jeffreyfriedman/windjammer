//! Code generation and emitted Rust file writing for single-file compilation.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analyzer::{AnalyzedFunction, SignatureRegistry};
use crate::codegen;
use crate::component_analyzer;
use crate::file_compiler::ModuleCompiler;
use crate::inference::InferredBounds;
use crate::metadata::{metadata_function_sig_from_analyzer, ModuleMetadata};
use crate::parser;
use crate::project_paths;
use crate::type_inference;
use crate::CompilationTarget;

/// `EarlySuccess` = Go / WASM component paths already wrote outputs.
pub(crate) enum MainCodegenOutcome {
    EarlySuccess,
    RustCode(String),
}

#[allow(clippy::too_many_arguments)] // Single entry point for full compile pipeline
pub(crate) fn generate_main_rust_code<'ast>(
    target: CompilationTarget,
    source_root: &Path,
    input_path: &Path,
    output_dir: &Path,
    module_compiler: &mut ModuleCompiler,
    is_multi_file_project: bool,
    program: &parser::Program<'ast>,
    analyzed: &[AnalyzedFunction<'ast>],
    signatures: &SignatureRegistry,
    analyzed_trait_methods: HashMap<String, HashMap<String, AnalyzedFunction<'ast>>>,
    inferred_bounds_map: HashMap<String, InferredBounds>,
    source: &str,
) -> Result<MainCodegenOutcome> {
    if target == CompilationTarget::Go {
        use codegen::backend::{CodegenConfig, Target};
        let config = CodegenConfig {
            target: Target::Go,
            output_dir: output_dir.to_path_buf(),
            ..Default::default()
        };

        let output = codegen::generate(program, Target::Go, Some(config))
            .map_err(|e| anyhow::anyhow!("Go codegen error: {}", e))?;

        let output_file = output_dir.join("main.go");
        std::fs::write(&output_file, &output.source)?;

        for (filename, content) in &output.additional_files {
            let file_path = output_dir.join(filename);
            std::fs::write(file_path, content)?;
        }

        return Ok(MainCodegenOutcome::EarlySuccess);
    }

    if target == CompilationTarget::Wasm {
        let mut comp_analyzer = component_analyzer::ComponentAnalyzer::new();
        let has_components = comp_analyzer.analyze(&program.items).is_ok()
            && comp_analyzer.all_components().next().is_some();

        if has_components {
            use codegen::backend::{CodegenConfig, Target};
            let config = CodegenConfig {
                target: Target::WebAssembly,
                output_dir: output_dir.to_path_buf(),
                ..Default::default()
            };

            let output = codegen::generate(program, Target::WebAssembly, Some(config))
                .map_err(|e| anyhow::anyhow!("Component codegen error: {}", e))?;

            let output_file = output_dir.join("lib.rs");
            std::fs::write(output_file, &output.source)?;

            for (filename, content) in &output.additional_files {
                let file_path = output_dir.join(filename);
                std::fs::write(file_path, content)?;
            }

            return Ok(MainCodegenOutcome::EarlySuccess);
        }

        let mut float_inference = type_inference::FloatInference::new();
        float_inference.set_source_root(source_root);
        float_inference.set_global_struct_field_types(&module_compiler.global_struct_field_types);
        float_inference.set_debug_source(source);
        float_inference.infer_program(program);

        if !float_inference.errors.is_empty() {
            eprintln!(
                "🚨 Float type inference errors in {}:",
                input_path.display()
            );
            for error in &float_inference.errors {
                eprintln!("  {}", error);
            }
            return Err(anyhow::anyhow!(
                "Type inference failed with {} error(s)",
                float_inference.errors.len()
            ));
        }

        let mut generator_signatures = module_compiler.global_signatures.clone();
        generator_signatures.merge(signatures);

        let mut generator = if is_multi_file_project {
            codegen::CodeGenerator::new_for_module(generator_signatures, target)
        } else {
            codegen::CodeGenerator::new(generator_signatures, target)
        };
        generator.set_float_inference(float_inference);
        generator.set_inferred_bounds(inferred_bounds_map);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);
        generator.set_global_struct_field_types(module_compiler.global_struct_field_types.clone());
        generator.set_copy_types_registry(module_compiler.copy_structs_registry.clone());

        generator.set_source_file(input_path);
        let output_file_path =
            project_paths::get_relative_output_path(source_root, input_path, output_dir)?;
        if let Some(parent) = output_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        generator.set_output_file(&output_file_path);

        if let Ok(cwd) = std::env::current_dir() {
            generator.set_workspace_root(cwd);
        }

        let result = generator.generate_program(program, analyzed);

        let source_map_path = output_file_path.with_extension("rs.map");
        if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
            eprintln!("Warning: Failed to save source map: {}", e);
        }

        return Ok(MainCodegenOutcome::RustCode(result));
    }

    // Rust target (and any non-Wasm non-Go path)
    let mut float_inference = type_inference::FloatInference::new();
    float_inference.set_source_root(source_root);
    float_inference.set_global_struct_field_types(&module_compiler.global_struct_field_types);
    float_inference.set_debug_source(source);
    float_inference.infer_program(program);

    if !float_inference.errors.is_empty() {
        eprintln!(
            "🚨 Float type inference errors in {}:",
            input_path.display()
        );
        for error in &float_inference.errors {
            eprintln!("  {}", error);
        }
        return Err(anyhow::anyhow!(
            "Type inference failed with {} error(s)",
            float_inference.errors.len()
        ));
    }

    let mut int_inference = type_inference::IntInference::new();
    int_inference.set_global_struct_field_types(&module_compiler.global_struct_field_types);
    int_inference.infer_program(program);
    if !int_inference.errors.is_empty() {
        eprintln!("🚨 Int type inference errors in {}:", input_path.display());
        for error in &int_inference.errors {
            eprintln!("  {}", error);
        }
        return Err(anyhow::anyhow!(
            "Int type inference failed with {} error(s)",
            int_inference.errors.len()
        ));
    }

    let mut generator_signatures = module_compiler.global_signatures.clone();
    generator_signatures.merge(signatures);
    let mut generator = if is_multi_file_project {
        codegen::CodeGenerator::new_for_module(generator_signatures, target)
    } else {
        codegen::CodeGenerator::new(generator_signatures, target)
    };
    generator.set_float_inference(float_inference);
    generator.set_int_inference(int_inference);
    generator.set_inferred_bounds(inferred_bounds_map);
    generator.set_analyzed_trait_methods(analyzed_trait_methods);
    generator.set_global_struct_field_types(module_compiler.global_struct_field_types.clone());
    generator.set_copy_types_registry(module_compiler.copy_structs_registry.clone());

    generator.set_source_file(input_path);
    let output_file_path =
        project_paths::get_relative_output_path(source_root, input_path, output_dir)?;
    if let Some(parent) = output_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    generator.set_output_file(&output_file_path);

    if let Ok(cwd) = std::env::current_dir() {
        generator.set_workspace_root(cwd);
    }

    let result = generator.generate_program(program, analyzed);

    let source_map_path = output_file_path.with_extension("rs.map");
    if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
        eprintln!("Warning: Failed to save source map: {}", e);
    }

    Ok(MainCodegenOutcome::RustCode(result))
}

#[allow(clippy::too_many_arguments)] // Mirrors `generate_main_rust_code` callers
pub(crate) fn write_single_file_outputs<'ast>(
    target: CompilationTarget,
    source_root: &Path,
    input_path: &Path,
    output_dir: &Path,
    module_compiler: &mut ModuleCompiler,
    is_multi_file_project: bool,
    program: &parser::Program<'ast>,
    signatures: &SignatureRegistry,
    rust_code: String,
) -> Result<(HashSet<String>, Vec<String>)> {
    let has_module_declarations = program
        .items
        .iter()
        .any(|item| matches!(item, parser::Item::Mod { items, .. } if items.is_empty()));

    let combined_code = if is_multi_file_project || has_module_declarations {
        rust_code
    } else {
        let module_code = module_compiler.get_compiled_modules().join("\n");
        if module_code.is_empty() {
            rust_code
        } else {
            format!("{}\n\n{}", module_code, rust_code)
        }
    };

    let output_file = project_paths::get_relative_output_path(source_root, input_path, output_dir)?;

    let is_lib_file = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s == "lib.wj")
        .unwrap_or(false);

    let is_output_subdirectory = {
        let components: Vec<_> = output_dir.components().collect();
        let mut found_src = false;
        for (i, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(name) = component {
                if name.to_string_lossy() == "src" && i + 1 < components.len() {
                    found_src = true;
                    break;
                }
            }
        }
        found_src
    };

    if is_lib_file && is_output_subdirectory {
        eprintln!(
            "⏭️  SKIPPING lib.rs generation in subdirectory: {}",
            output_dir.display()
        );
        eprintln!("   lib.rs should only exist at crate root, not in subdirectories");
        return Ok((HashSet::new(), Vec::new()));
    }

    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    eprintln!(
        "📝 WRITING FILE: {} ({} bytes)",
        output_file.display(),
        combined_code.len()
    );
    if combined_code.is_empty() {
        eprintln!("    🚨 WARNING: Writing EMPTY file!");
        eprintln!("    rust_code length: check generator output");
    }

    {
        use std::io::Write;
        let mut file = std::fs::File::create(&output_file)?;
        file.write_all(combined_code.as_bytes())?;
        file.flush()?;

        #[cfg(target_os = "linux")]
        {
            file.sync_all()?;
            drop(file);
            if let Some(parent) = output_file.parent() {
                let dir = std::fs::File::open(parent)?;
                dir.sync_all()?;
            }
        }

        #[cfg(not(target_os = "linux"))]
        drop(file);
    }

    if target == CompilationTarget::Rust {
        let module_path = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let mut meta = ModuleMetadata::new(module_path.to_string());

        for item in &program.items {
            match item {
                parser::Item::Function { decl, .. } => {
                    if let Some(sig) = signatures.get_signature(&decl.name) {
                        meta.functions.insert(
                            decl.name.clone(),
                            metadata_function_sig_from_analyzer(sig, false, None),
                        );
                    }
                }
                parser::Item::Impl { block, .. } => {
                    let type_name = &block.type_name;
                    for func_decl in &block.functions {
                        let full_name = format!("{}::{}", type_name, func_decl.name);
                        if let Some(sig) = signatures.get_signature(&full_name) {
                            meta.functions.insert(
                                full_name,
                                metadata_function_sig_from_analyzer(
                                    sig,
                                    true,
                                    Some(type_name.clone()),
                                ),
                            );
                        }
                    }
                }
                parser::Item::Struct { decl, .. } => {
                    let mut fields = std::collections::HashMap::new();
                    for field in &decl.fields {
                        fields.insert(
                            field.name.clone(),
                            ModuleMetadata::serialize_type(&field.field_type),
                        );
                    }
                    meta.structs.insert(decl.name.clone(), fields);
                }
                _ => {}
            }
        }

        let meta_path = crate::metadata::meta_cache_path(input_path);
        if let Some(parent) = meta_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let meta_json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(&meta_path, &meta_json)?;

        eprintln!(
            "📋 METADATA: {} ({} functions, {} structs)",
            meta_path.display(),
            meta.functions.len(),
            meta.structs.len()
        );
    }

    Ok((
        module_compiler.imported_stdlib_modules.clone(),
        module_compiler.external_crates.clone(),
    ))
}
