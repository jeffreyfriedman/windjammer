//! Inline Variable/Function refactoring
//!
//! Replaces all usages of a variable or function with its definition.

use super::ast_utils;
use crate::database::WindjammerDatabase;
use tower_lsp::lsp_types::*;

/// Inline a variable or function
pub struct InlineRefactoring<'a> {
    db: &'a WindjammerDatabase,
    uri: Url,
    position: Position,
}

/// Result of analyzing a variable for inlining
#[derive(Debug, Clone)]
pub struct InlineAnalysis {
    /// Name of the variable to inline
    pub name: String,
    /// The value/expression to inline
    pub value: String,
    /// Range of the variable definition
    pub definition_range: Range,
    /// Ranges of all usages of the variable
    pub usage_ranges: Vec<Range>,
    /// Whether it's safe to inline
    pub is_safe: bool,
    /// Reason if not safe
    pub unsafe_reason: Option<String>,
}

impl<'a> InlineRefactoring<'a> {
    /// Create a new inline refactoring
    pub fn new(db: &'a WindjammerDatabase, uri: Url, position: Position) -> Self {
        Self { db, uri, position }
    }

    /// Execute the refactoring
    pub fn execute(&self, source: &str) -> Result<WorkspaceEdit, String> {
        // Step 1: Analyze what's at the cursor
        let analysis = self.analyze_variable(source)?;

        // Step 2: Safety checks
        if !analysis.is_safe {
            return Err(analysis
                .unsafe_reason
                .unwrap_or_else(|| "Cannot inline: unsafe".to_string()));
        }

        // Step 3: Create text edits
        let mut edits = vec![];

        // Replace all usages with the value
        for usage_range in &analysis.usage_ranges {
            edits.push(TextEdit {
                range: *usage_range,
                new_text: analysis.value.clone(),
            });
        }

        // Remove the variable definition
        edits.push(TextEdit {
            range: analysis.definition_range,
            new_text: String::new(),
        });

        // Step 4: Create workspace edit
        let mut changes = std::collections::HashMap::new();
        changes.insert(self.uri.clone(), edits);

        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Analyze the variable at the cursor position
    fn analyze_variable(&self, source: &str) -> Result<InlineAnalysis, String> {
        // Find the word at the cursor position
        let byte_offset = ast_utils::position_to_byte_offset(source, self.position);
        let word = self.extract_word_at_offset(source, byte_offset)?;

        // Find the definition
        let (def_range, value) = self.find_definition(source, &word)?;

        // Find all usages
        let usage_ranges = self.find_usages(source, &word, def_range);

        // Check if it's safe to inline
        let (is_safe, unsafe_reason) = self.check_safety(source, &word, &value);

        Ok(InlineAnalysis {
            name: word,
            value,
            definition_range: def_range,
            usage_ranges,
            is_safe,
            unsafe_reason,
        })
    }

    /// Extract the word at the given byte offset
    fn extract_word_at_offset(&self, source: &str, offset: usize) -> Result<String, String> {
        let bytes = source.as_bytes();
        if offset >= bytes.len() {
            return Err("Position out of bounds".to_string());
        }

        // Find word boundaries
        let mut start = offset;
        let mut end = offset;

        // Expand left
        while start > 0 {
            let ch = bytes[start - 1] as char;
            if ch.is_alphanumeric() || ch == '_' {
                start -= 1;
            } else {
                break;
            }
        }

        // Expand right
        while end < bytes.len() {
            let ch = bytes[end] as char;
            if ch.is_alphanumeric() || ch == '_' {
                end += 1;
            } else {
                break;
            }
        }

        if start == end {
            return Err("No identifier at cursor".to_string());
        }

        Ok(source[start..end].to_string())
    }

    /// Find the definition of a variable
    fn find_definition(&self, source: &str, name: &str) -> Result<(Range, String), String> {
        // Simple regex-based search for "let name = value"
        let pattern = format!(r"let\s+{}\s*=\s*([^;\n]+)", regex::escape(name));
        let re = regex::Regex::new(&pattern).map_err(|e| e.to_string())?;

        if let Some(captures) = re.captures(source) {
            let full_match = captures.get(0).unwrap();
            let value_match = captures.get(1).unwrap();

            let start_pos = ast_utils::byte_offset_to_position(source, full_match.start());
            let end_pos = ast_utils::byte_offset_to_position(source, full_match.end());

            // Include the newline in the deletion
            let end_line = end_pos.line + 1;
            let range = Range {
                start: Position {
                    line: start_pos.line,
                    character: 0,
                },
                end: Position {
                    line: end_line,
                    character: 0,
                },
            };

            Ok((range, value_match.as_str().trim().to_string()))
        } else {
            Err(format!("Could not find definition of '{}'", name))
        }
    }

    /// Find all usages of a variable
    fn find_usages(&self, source: &str, name: &str, def_range: Range) -> Vec<Range> {
        let mut usages = vec![];
        let def_start_byte = ast_utils::position_to_byte_offset(source, def_range.start);
        let def_end_byte = ast_utils::position_to_byte_offset(source, def_range.end);

        // Find all occurrences of the name
        let pattern = format!(r"\b{}\b", regex::escape(name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            for m in re.find_iter(source) {
                let match_start = m.start();
                let match_end = m.end();

                // Skip the definition itself
                if match_start >= def_start_byte && match_end <= def_end_byte {
                    continue;
                }

                let start_pos = ast_utils::byte_offset_to_position(source, match_start);
                let end_pos = ast_utils::byte_offset_to_position(source, match_end);

                usages.push(Range {
                    start: start_pos,
                    end: end_pos,
                });
            }
        }

        usages
    }

    /// Check if it's safe to inline
    fn check_safety(&self, _source: &str, _name: &str, value: &str) -> (bool, Option<String>) {
        // Safety check 1: Don't inline complex expressions with side effects
        if value.contains("(") && (value.contains("!") || value.contains("await")) {
            return (
                false,
                Some("Cannot inline: expression may have side effects".to_string()),
            );
        }

        // Safety check 2: Value should be simple enough
        if value.len() > 100 {
            return (
                false,
                Some("Cannot inline: expression too complex".to_string()),
            );
        }

        // Safety check 3: No assignments in the value
        if value.contains("=") && !value.contains("==") && !value.contains("!=") {
            return (
                false,
                Some("Cannot inline: expression contains assignment".to_string()),
            );
        }

        (true, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_word_at_offset() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let position = Position {
            line: 0,
            character: 10,
        };
        let inline = InlineRefactoring::new(&db, uri, position);

        let source = "let x = 42";
        // Position at 'x' (byte 4)
        let word = inline.extract_word_at_offset(source, 4).unwrap();
        assert_eq!(word, "x");
    }

    #[test]
    fn test_find_definition() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let position = Position {
            line: 0,
            character: 0,
        };
        let inline = InlineRefactoring::new(&db, uri, position);

        let source = "let x = 42\nlet y = x + 1";
        let (range, value) = inline.find_definition(source, "x").unwrap();

        assert_eq!(value, "42");
        assert_eq!(range.start.line, 0);
    }

    #[test]
    fn test_find_usages() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let position = Position {
            line: 0,
            character: 0,
        };
        let inline = InlineRefactoring::new(&db, uri, position);

        let source = "let x = 42\nlet y = x + 1\nlet z = x * 2";
        let def_range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 0,
            },
        };

        let usages = inline.find_usages(source, "x", def_range);

        // Should find 2 usages (in lines 2 and 3)
        assert_eq!(usages.len(), 2);
    }
}
