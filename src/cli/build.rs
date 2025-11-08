// wj build - Build Windjammer project
//
// This command compiles Windjammer source files to Rust.

use anyhow::Result;
use colored::*;
use std::path::Path;

/// Build options for JavaScript target
pub struct BuildOptions {
    pub minify: bool,
    pub tree_shake: bool,
    pub source_maps: bool,
    pub polyfills: bool,
    pub v8_optimize: bool,
}

pub fn execute(
    path: &Path,
    output: Option<&Path>,
    _release: bool,
    target_str: &str,
    options: BuildOptions,
) -> Result<()> {
    let output_dir = output.unwrap_or_else(|| Path::new("./build"));

    println!(
        "{} Windjammer project from {:?} (target: {})",
        "Building".green().bold(),
        path,
        target_str
    );
    println!("Output: {:?}", output_dir);

    // Parse target string
    let target = match target_str.to_lowercase().as_str() {
        "rust" => crate::CompilationTarget::Rust,
        "javascript" | "js" => {
            // Use new JavaScript backend
            use crate::codegen::backend::{CodegenConfig, Target};
            let config = CodegenConfig {
                target: Target::JavaScript,
                output_dir: output_dir.to_path_buf(),
                minify: options.minify,
                tree_shake: options.tree_shake,
                source_maps: options.source_maps,
                polyfills: options.polyfills,
                v8_optimize: options.v8_optimize,
                ..Default::default()
            };
            return build_javascript(path, &config);
        }
        "wasm" | "webassembly" => crate::CompilationTarget::Wasm,
        _ => {
            anyhow::bail!(
                "Unknown target: {}. Use 'rust', 'javascript', or 'wasm'",
                target_str
            );
        }
    };

    crate::build_project(path, output_dir, target)?;

    println!("\n{} Build complete!", "Success!".green().bold());
    if target_str == "javascript" || target_str == "js" {
        println!("Run your JavaScript project with:");
        println!("  node {:?}/output.js", output_dir);
    } else {
        println!("Run your project with:");
        println!("  cd {:?} && cargo run", output_dir);
    }

    Ok(())
}

fn build_javascript(path: &Path, config: &crate::codegen::backend::CodegenConfig) -> Result<()> {
    use crate::codegen;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::fs;

    // Read source file
    let source = fs::read_to_string(path)?;

    // Lex and parse
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    // Generate JavaScript
    let output = codegen::generate(&program, config.target, Some(config.clone()))?;

    // Create output directory
    fs::create_dir_all(&config.output_dir)?;

    // Write main output
    let output_path = config.output_dir.join("output.js");
    fs::write(&output_path, &output.source)?;
    println!("  {} {:?}", "Generated".green(), output_path);

    // Write TypeScript definitions if available
    if let Some(ref type_defs) = output.type_definitions {
        let types_path = config.output_dir.join("output.d.ts");
        fs::write(&types_path, type_defs)?;
        println!("  {} {:?}", "Generated".green(), types_path);
    }

    // Write additional files (package.json, etc.)
    for (filename, content) in &output.additional_files {
        let file_path = config.output_dir.join(filename);
        fs::write(&file_path, content)?;
        println!("  {} {:?}", "Generated".green(), file_path);
    }

    Ok(())
}
