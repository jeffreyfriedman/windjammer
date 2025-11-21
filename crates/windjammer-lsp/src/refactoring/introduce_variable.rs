#![allow(dead_code)] // Refactoring implementation - some parts planned for future versions
//! Introduce Variable refactoring
//!
//! Extracts an expression into a new variable with automatic naming.

use super::ast_utils;
use crate::database::WindjammerDatabase;
use tower_lsp::lsp_types::*;

/// Introduce a new variable from an expression
pub struct IntroduceVariable<'a> {
    db: &'a WindjammerDatabase,
    uri: Url,
    range: Range,
}

/// Result of analyzing an expression for variable introduction
#[derive(Debug, Clone)]
pub struct IntroduceAnalysis {
    /// The expression to extract
    pub expression: String,
    /// Suggested variable name
    pub suggested_name: String,
    /// Range of the expression to replace
    pub expression_range: Range,
    /// Position where to insert the new variable
    pub insert_position: Position,
    /// Other occurrences of the same expression
    pub duplicate_ranges: Vec<Range>,
    /// Whether it's safe to introduce
    pub is_safe: bool,
    /// Reason if not safe
    pub unsafe_reason: Option<String>,
}

impl<'a> IntroduceVariable<'a> {
    /// Create a new introduce variable refactoring
    pub fn new(db: &'a WindjammerDatabase, uri: Url, range: Range) -> Self {
        Self { db, uri, range }
    }

    /// Execute the refactoring
    pub fn execute(&self, variable_name: &str, source: &str) -> Result<WorkspaceEdit, String> {
        // Step 1: Analyze the selected expression
        let analysis = self.analyze_expression(source)?;

        // Step 2: Safety checks
        if !analysis.is_safe {
            return Err(analysis
                .unsafe_reason
                .unwrap_or_else(|| "Cannot introduce variable: unsafe".to_string()));
        }

        // Step 3: Create text edits
        let mut edits = vec![];

        // Determine the variable name to use
        let name = if variable_name.is_empty() {
            &analysis.suggested_name
        } else {
            variable_name
        };

        // Insert variable declaration
        let declaration = format!("let {} = {}\n    ", name, analysis.expression);
        edits.push(TextEdit {
            range: Range {
                start: analysis.insert_position,
                end: analysis.insert_position,
            },
            new_text: declaration,
        });

        // Replace the original expression with the variable name
        edits.push(TextEdit {
            range: analysis.expression_range,
            new_text: name.to_string(),
        });

        // Optionally replace duplicate expressions
        for dup_range in &analysis.duplicate_ranges {
            edits.push(TextEdit {
                range: *dup_range,
                new_text: name.to_string(),
            });
        }

        // Step 4: Create workspace edit
        let mut changes = std::collections::HashMap::new();
        changes.insert(self.uri.clone(), edits);

        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Analyze the selected expression
    fn analyze_expression(&self, source: &str) -> Result<IntroduceAnalysis, String> {
        // Extract the selected text
        let start_byte = ast_utils::position_to_byte_offset(source, self.range.start);
        let end_byte = ast_utils::position_to_byte_offset(source, self.range.end);

        if start_byte >= end_byte {
            return Err("Invalid selection".to_string());
        }

        let expression = source[start_byte..end_byte].trim().to_string();

        if expression.is_empty() {
            return Err("Selection is empty".to_string());
        }

        // Generate a suggested variable name
        let suggested_name = self.suggest_name(&expression);

        // Find where to insert the variable declaration
        let insert_position = self.find_insert_position(source, self.range.start)?;

        // Find duplicate expressions
        let duplicate_ranges = self.find_duplicates(source, &expression, self.range);

        // Check if it's safe to introduce
        let (is_safe, unsafe_reason) = self.check_safety(&expression);

        Ok(IntroduceAnalysis {
            expression,
            suggested_name,
            expression_range: self.range,
            insert_position,
            duplicate_ranges,
            is_safe,
            unsafe_reason,
        })
    }

    /// Suggest a variable name based on the expression
    fn suggest_name(&self, expression: &str) -> String {
        // Simple heuristic: try to extract meaningful words

        // If it's a simple literal, use a generic name
        if expression.parse::<i64>().is_ok() || expression.parse::<f64>().is_ok() {
            return "value".to_string();
        }

        // If it's a string literal
        if expression.starts_with('"') {
            return "text".to_string();
        }

        // If it contains an operator, suggest based on operation
        if expression.contains('+') {
            return "sum".to_string();
        }
        if expression.contains('-') {
            return "difference".to_string();
        }
        if expression.contains('*') {
            return "product".to_string();
        }
        if expression.contains('/') {
            return "quotient".to_string();
        }

        // If it's a function call, extract function name
        if let Some(paren_pos) = expression.find('(') {
            let func_name = expression[..paren_pos].trim();
            if !func_name.is_empty() && func_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return format!("{}_result", func_name);
            }
        }

        // If it's a field access, use the field name
        if let Some(dot_pos) = expression.rfind('.') {
            let field_name = expression[dot_pos + 1..].trim();
            if !field_name.is_empty() && field_name.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                return field_name.to_string();
            }
        }

        // Default name
        "temp".to_string()
    }

    /// Find where to insert the variable declaration
    fn find_insert_position(
        &self,
        _source: &str,
        selection_start: Position,
    ) -> Result<Position, String> {
        // Insert at the beginning of the line where the expression is
        Ok(Position {
            line: selection_start.line,
            character: 0,
        })
    }

    /// Find duplicate occurrences of the same expression
    fn find_duplicates(&self, source: &str, expression: &str, original_range: Range) -> Vec<Range> {
        let mut duplicates = vec![];
        let original_start = ast_utils::position_to_byte_offset(source, original_range.start);
        let original_end = ast_utils::position_to_byte_offset(source, original_range.end);

        // Find all occurrences of the expression
        let mut start = 0;
        while let Some(pos) = source[start..].find(expression) {
            let actual_pos = start + pos;
            let end_pos = actual_pos + expression.len();

            // Skip the original selection
            if actual_pos == original_start && end_pos == original_end {
                start = end_pos;
                continue;
            }

            // Check if it's a complete match (not part of a larger token)
            let before_ok =
                actual_pos == 0 || !source.as_bytes()[actual_pos - 1].is_ascii_alphanumeric();
            let after_ok =
                end_pos >= source.len() || !source.as_bytes()[end_pos].is_ascii_alphanumeric();

            if before_ok && after_ok {
                let start_pos = ast_utils::byte_offset_to_position(source, actual_pos);
                let end_pos_lsp = ast_utils::byte_offset_to_position(source, end_pos);

                duplicates.push(Range {
                    start: start_pos,
                    end: end_pos_lsp,
                });
            }

            start = end_pos;
        }

        duplicates
    }

    /// Check if it's safe to introduce a variable
    fn check_safety(&self, expression: &str) -> (bool, Option<String>) {
        // Safety check 1: Expression should not be empty
        if expression.trim().is_empty() {
            return (false, Some("Expression is empty".to_string()));
        }

        // Safety check 2: Expression should not be just a variable name
        // (no point in introducing a variable for a variable)
        if expression.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return (false, Some("Selection is already a variable".to_string()));
        }

        // Safety check 3: Don't introduce for simple literals unless it's used multiple times
        if expression.parse::<i64>().is_ok()
            || expression.parse::<f64>().is_ok()
            || (expression.starts_with('"') && expression.ends_with('"'))
        {
            // This is okay if used multiple times (will be handled by duplicate detection)
        }

        (true, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_name_arithmetic() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 5,
            },
        };
        let introduce = IntroduceVariable::new(&db, uri, range);

        assert_eq!(introduce.suggest_name("a + b"), "sum");
        assert_eq!(introduce.suggest_name("x * y"), "product");
        assert_eq!(introduce.suggest_name("x - y"), "difference");
        assert_eq!(introduce.suggest_name("a / b"), "quotient");
    }

    #[test]
    fn test_suggest_name_function_call() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 10,
            },
        };
        let introduce = IntroduceVariable::new(&db, uri, range);

        assert_eq!(introduce.suggest_name("calculate()"), "calculate_result");
        assert_eq!(introduce.suggest_name("get_value(x)"), "get_value_result");
    }

    #[test]
    fn test_suggest_name_field_access() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 10,
            },
        };
        let introduce = IntroduceVariable::new(&db, uri, range);

        assert_eq!(introduce.suggest_name("obj.name"), "name");
        assert_eq!(introduce.suggest_name("user.age"), "age");
    }

    #[test]
    fn test_find_duplicates() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 8,
            },
            end: Position {
                line: 0,
                character: 13,
            },
        };
        let introduce = IntroduceVariable::new(&db, uri, range);

        let source = "let a = x + y\nlet b = x + y\nlet c = x + y";
        let duplicates = introduce.find_duplicates(source, "x + y", range);

        // Should find 2 duplicates (lines 2 and 3)
        assert_eq!(duplicates.len(), 2);
    }

    #[test]
    fn test_check_safety_simple_variable() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        };
        let introduce = IntroduceVariable::new(&db, uri, range);

        let (is_safe, reason) = introduce.check_safety("x");
        assert!(!is_safe, "Should reject simple variable names");
        assert!(reason.unwrap().contains("already a variable"));
    }
}
