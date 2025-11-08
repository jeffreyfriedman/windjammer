// Auto-Fix System
//
// Detects fixable errors and generates code fixes automatically.
//
// Supported fixes:
// 1. Add `let mut` for immutability errors
// 2. Add missing imports
// 3. Fix typos in variable/function names (fuzzy matching)
// 4. Add `.parse()` for string-to-int conversions
// 5. Add `.to_string()` for &str to String conversions

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// FIX TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FixType {
    /// Add `mut` keyword to variable declaration
    AddMut {
        file: PathBuf,
        line: usize,
        variable_name: String,
    },
    /// Add missing import statement
    AddImport {
        file: PathBuf,
        module_path: String,
    },
    /// Fix typo in identifier (suggest correct spelling)
    FixTypo {
        file: PathBuf,
        line: usize,
        column: usize,
        wrong_name: String,
        correct_name: String,
    },
    /// Add `.parse()` call for type conversion
    AddParse {
        file: PathBuf,
        line: usize,
        column: usize,
        expression: String,
    },
    /// Add `.to_string()` call for &str to String conversion
    AddToString {
        file: PathBuf,
        line: usize,
        column: usize,
        expression: String,
    },
}

impl FixType {
    /// Get a human-readable description of this fix
    pub fn description(&self) -> String {
        match self {
            FixType::AddMut { variable_name, .. } => {
                format!("Add 'mut' to variable '{}'", variable_name)
            }
            FixType::AddImport { module_path, .. } => {
                format!("Add import for '{}'", module_path)
            }
            FixType::FixTypo {
                wrong_name,
                correct_name,
                ..
            } => {
                format!("Replace '{}' with '{}'", wrong_name, correct_name)
            }
            FixType::AddParse { expression, .. } => {
                format!("Add .parse() to '{}'", expression)
            }
            FixType::AddToString { expression, .. } => {
                format!("Add .to_string() to '{}'", expression)
            }
        }
    }
}

// ============================================================================
// FIX DETECTOR
// ============================================================================

pub struct FixDetector {
    /// Map of error codes to fix detection functions
    detectors: HashMap<String, Box<dyn Fn(&str, &str) -> Option<FixType>>>,
}

impl FixDetector {
    pub fn new() -> Self {
        let mut detectors: HashMap<String, Box<dyn Fn(&str, &str) -> Option<FixType>>> =
            HashMap::new();

        // E0384: cannot assign twice to immutable variable
        detectors.insert(
            "E0384".to_string(),
            Box::new(|error_msg: &str, _file_content: &str| {
                // Extract variable name from error message
                if let Some(var_name) = extract_variable_name(error_msg) {
                    // TODO: Extract file and line from error
                    Some(FixType::AddMut {
                        file: PathBuf::from("unknown"),
                        line: 0,
                        variable_name: var_name,
                    })
                } else {
                    None
                }
            }),
        );

        // E0308: mismatched types (string to int)
        detectors.insert(
            "E0308".to_string(),
            Box::new(|error_msg: &str, _file_content: &str| {
                if error_msg.contains("expected int") && error_msg.contains("found string") {
                    // TODO: Extract expression and location
                    Some(FixType::AddParse {
                        file: PathBuf::from("unknown"),
                        line: 0,
                        column: 0,
                        expression: "value".to_string(),
                    })
                } else if error_msg.contains("expected String") && error_msg.contains("found &str")
                {
                    Some(FixType::AddToString {
                        file: PathBuf::from("unknown"),
                        line: 0,
                        column: 0,
                        expression: "value".to_string(),
                    })
                } else {
                    None
                }
            }),
        );

        Self { detectors }
    }

    /// Detect fixable errors from a Rust error message
    pub fn detect_fixes(&self, error_code: &str, error_msg: &str, file_content: &str) -> Vec<FixType> {
        let mut fixes = Vec::new();

        if let Some(detector) = self.detectors.get(error_code) {
            if let Some(fix) = detector(error_msg, file_content) {
                fixes.push(fix);
            }
        }

        fixes
    }
}

// ============================================================================
// FIX APPLICATOR
// ============================================================================

pub struct FixApplicator;

impl FixApplicator {
    pub fn new() -> Self {
        Self
    }

    /// Apply a fix to a source file
    pub fn apply_fix(&self, fix: &FixType) -> Result<()> {
        use std::fs;

        match fix {
            FixType::AddMut {
                file,
                line,
                variable_name,
            } => {
                let content = fs::read_to_string(file)?;
                let lines: Vec<&str> = content.lines().collect();

                if *line == 0 || *line > lines.len() {
                    anyhow::bail!("Invalid line number: {}", line);
                }

                let target_line = lines[*line - 1];

                // Find "let variable_name" and replace with "let mut variable_name"
                let pattern = format!("let {}", variable_name);
                let replacement = format!("let mut {}", variable_name);

                if target_line.contains(&pattern) {
                    let new_line = target_line.replace(&pattern, &replacement);
                    let mut new_lines = lines.clone();
                    new_lines[*line - 1] = &new_line;

                    let new_content = new_lines.join("\n");
                    fs::write(file, new_content)?;

                    println!("✓ Applied fix: {}", fix.description());
                    Ok(())
                } else {
                    anyhow::bail!("Could not find pattern '{}' in line {}", pattern, line);
                }
            }
            FixType::AddImport { file, module_path } => {
                let content = fs::read_to_string(file)?;
                let import_statement = format!("use {}\n", module_path);

                // Add import at the top of the file (after any existing imports)
                let new_content = if content.starts_with("use ") {
                    // Find the last import line
                    let lines: Vec<&str> = content.lines().collect();
                    let mut last_import_idx = 0;
                    for (idx, line) in lines.iter().enumerate() {
                        if line.starts_with("use ") {
                            last_import_idx = idx;
                        } else if !line.trim().is_empty() {
                            break;
                        }
                    }

                    let mut new_lines = lines.clone();
                    new_lines.insert(last_import_idx + 1, &import_statement);
                    new_lines.join("\n")
                } else {
                    // No existing imports, add at the top
                    format!("{}{}", import_statement, content)
                };

                fs::write(file, new_content)?;
                println!("✓ Applied fix: {}", fix.description());
                Ok(())
            }
            FixType::FixTypo {
                file,
                line,
                column: _,
                wrong_name,
                correct_name,
            } => {
                let content = fs::read_to_string(file)?;
                let lines: Vec<&str> = content.lines().collect();

                if *line == 0 || *line > lines.len() {
                    anyhow::bail!("Invalid line number: {}", line);
                }

                let target_line = lines[*line - 1];

                // Replace wrong_name with correct_name
                if target_line.contains(wrong_name) {
                    let new_line = target_line.replace(wrong_name, correct_name);
                    let mut new_lines = lines.clone();
                    new_lines[*line - 1] = &new_line;

                    let new_content = new_lines.join("\n");
                    fs::write(file, new_content)?;

                    println!("✓ Applied fix: {}", fix.description());
                    Ok(())
                } else {
                    anyhow::bail!("Could not find '{}' in line {}", wrong_name, line);
                }
            }
            FixType::AddParse {
                file,
                line,
                column: _,
                expression,
            } => {
                let content = fs::read_to_string(file)?;
                let lines: Vec<&str> = content.lines().collect();

                if *line == 0 || *line > lines.len() {
                    anyhow::bail!("Invalid line number: {}", line);
                }

                let target_line = lines[*line - 1];

                // Add .parse() to the expression
                let new_line = target_line.replace(expression, &format!("{}.parse()", expression));
                let mut new_lines = lines.clone();
                new_lines[*line - 1] = &new_line;

                let new_content = new_lines.join("\n");
                fs::write(file, new_content)?;

                println!("✓ Applied fix: {}", fix.description());
                Ok(())
            }
            FixType::AddToString {
                file,
                line,
                column: _,
                expression,
            } => {
                let content = fs::read_to_string(file)?;
                let lines: Vec<&str> = content.lines().collect();

                if *line == 0 || *line > lines.len() {
                    anyhow::bail!("Invalid line number: {}", line);
                }

                let target_line = lines[*line - 1];

                // Add .to_string() to the expression
                let new_line =
                    target_line.replace(expression, &format!("{}.to_string()", expression));
                let mut new_lines = lines.clone();
                new_lines[*line - 1] = &new_line;

                let new_content = new_lines.join("\n");
                fs::write(file, new_content)?;

                println!("✓ Applied fix: {}", fix.description());
                Ok(())
            }
        }
    }

    /// Apply multiple fixes to source files
    pub fn apply_fixes(&self, fixes: &[FixType]) -> Result<()> {
        for fix in fixes {
            self.apply_fix(fix)?;
        }
        Ok(())
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extract variable name from error message
fn extract_variable_name(error_msg: &str) -> Option<String> {
    // Look for patterns like "cannot assign twice to immutable variable `x`"
    if let Some(start) = error_msg.find('`') {
        if let Some(end) = error_msg[start + 1..].find('`') {
            return Some(error_msg[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variable_name() {
        let msg = "cannot assign twice to immutable variable `x`";
        assert_eq!(extract_variable_name(msg), Some("x".to_string()));

        let msg2 = "cannot assign twice to immutable variable `my_var`";
        assert_eq!(extract_variable_name(msg2), Some("my_var".to_string()));
    }

    #[test]
    fn test_fix_description() {
        let fix = FixType::AddMut {
            file: PathBuf::from("test.wj"),
            line: 5,
            variable_name: "x".to_string(),
        };
        assert_eq!(fix.description(), "Add 'mut' to variable 'x'");
    }
}

