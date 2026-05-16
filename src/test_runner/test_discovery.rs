//! Discovering test files and parsing `test_*` functions from Windjammer sources.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Test function metadata
#[derive(Debug, Clone)]
pub(crate) struct TestFunction {
    pub(crate) name: String,
    pub(crate) file: PathBuf,
}

/// Discover test files in a directory
pub(crate) fn discover_test_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    if dir.is_file() {
        // Single file
        if is_test_file(dir) {
            test_files.push(dir.to_path_buf());
        }
    } else {
        // Directory - search recursively
        visit_dirs(dir, &mut test_files)?;
    }

    Ok(test_files)
}

/// Visit directories recursively to find test files
fn visit_dirs(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<()> {
    use std::fs;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip target, build, and hidden directories
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || name_str == "target" || name_str == "build" {
                        continue;
                    }
                }
                visit_dirs(&path, test_files)?;
            } else if is_test_file(&path) {
                test_files.push(path);
            }
        }
    }

    Ok(())
}

/// Check if a file is a test file
/// TDD FIX: Only discover test files in tests_wj/ directories or files ending in _test.wj
/// THE WINDJAMMER WAY: Avoid false positives by checking directory structure
fn is_test_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();

        // Must end with .wj
        if !name_str.ends_with(".wj") {
            return false;
        }

        // Check if file is in tests/ directory OR ends with _test.wj
        let in_tests_dir = path
            .components()
            .any(|c| c.as_os_str().to_string_lossy() == "tests");

        let ends_with_test = name_str.ends_with("_test.wj");

        in_tests_dir || ends_with_test
    } else {
        false
    }
}

/// Compile a test file and extract test functions
pub(crate) fn compile_test_file(test_file: &Path, _output_dir: &Path) -> Result<Vec<TestFunction>> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::fs;

    let source = fs::read_to_string(test_file)?;

    // Lex and parse
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);

    // TDD DEBUG: Add file context to parser errors
    let program = parser.parse().map_err(|e| {
        eprintln!("DEBUG: Parser error in file: {}", test_file.display());
        eprintln!("DEBUG: Error message: {}", e);
        anyhow::anyhow!("In file {}: {}", test_file.display(), e)
    })?;

    // Find test functions
    let mut tests = Vec::new();
    for item in &program.items {
        if let crate::parser::Item::Function { decl: func, .. } = item {
            if func.name.starts_with("test_") {
                tests.push(TestFunction {
                    name: func.name.clone(),
                    file: test_file.to_path_buf(),
                });
            }
        }
    }

    Ok(tests)
}
