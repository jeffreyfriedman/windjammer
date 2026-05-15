//! Mutability and other per-file compile error checks for `file_compiler`.

use anyhow::Result;
use std::path::Path;

use crate::errors;
use crate::parser;

/// Used by `compile_file_impl` after parsing the main file.
pub(crate) fn check_top_level_mutability(input_path: &Path, program: &parser::Program) -> Result<()> {
    let mut mut_checker = errors::MutabilityChecker::new(input_path.to_path_buf());
    let mut has_mut_errors = false;
    apply_mutability_check(&mut mut_checker, program, &mut has_mut_errors);
    if has_mut_errors {
        anyhow::bail!("Compilation failed: mutability errors detected");
    }
    Ok(())
}

/// Used by `ModuleCompiler::compile_module` for dependency modules.
pub(crate) fn check_module_mutability(
    module_path: &str,
    file_path: &Path,
    program: &parser::Program,
) -> Result<()> {
    let mut mut_checker = errors::MutabilityChecker::new(file_path.to_path_buf());
    let mut has_mut_errors = false;
    apply_mutability_check(&mut mut_checker, program, &mut has_mut_errors);
    if has_mut_errors {
        anyhow::bail!(
            "Compilation failed: mutability errors detected in module '{}'",
            module_path
        );
    }
    Ok(())
}

fn apply_mutability_check(
    mut_checker: &mut errors::MutabilityChecker,
    program: &parser::Program,
    has_mut_errors: &mut bool,
) {
    for item in &program.items {
        match item {
            parser::Item::Function { decl, .. } => {
                let mut_errors = mut_checker.check_function(decl);
                if !mut_errors.is_empty() {
                    *has_mut_errors = true;
                    for error in &mut_errors {
                        eprintln!("{}", error.format_error());
                    }
                }
            }
            parser::Item::Impl { block, .. } => {
                for func_decl in &block.functions {
                    let mut_errors = mut_checker.check_function(func_decl);
                    if !mut_errors.is_empty() {
                        *has_mut_errors = true;
                        for error in &mut_errors {
                            eprintln!("{}", error.format_error());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
