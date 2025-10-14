//! Extract Function refactoring
//!
//! Transforms selected code into a new function with automatic parameter
//! and return value detection.

use super::ast_utils;
use super::scope_analyzer::ScopeAnalysis;
use crate::database::WindjammerDatabase;
use tower_lsp::lsp_types::*;
use windjammer::parser::Parameter;

/// Extract selected code into a new function
pub struct ExtractFunction<'a> {
    #[allow(dead_code)]
    db: &'a WindjammerDatabase,
    uri: Url,
    range: Range,
}

impl<'a> ExtractFunction<'a> {
    /// Create a new extract function refactoring
    pub fn new(db: &'a WindjammerDatabase, uri: Url, range: Range) -> Self {
        Self { db, uri, range }
    }

    /// Execute the refactoring
    pub fn execute(&self, function_name: &str, source: &str) -> Result<WorkspaceEdit, String> {
        // Step 1: Extract the selected text
        let start_byte = ast_utils::position_to_byte_offset(source, self.range.start);
        let end_byte = ast_utils::position_to_byte_offset(source, self.range.end);
        let selected_text = &source[start_byte..end_byte];

        if selected_text.trim().is_empty() {
            return Err("Selection is empty".to_string());
        }

        // Step 2: Analyze scope (simplified for now - will integrate with parser later)
        let analysis = self.analyze_scope()?;

        // Step 3: Generate the new function
        let new_function = self.generate_function(function_name, &analysis, selected_text);

        // Step 4: Generate the function call
        let function_call = self.generate_call(function_name, &analysis);

        // Step 5: Create text edits
        let mut edits = vec![];

        // Edit 1: Replace selection with function call
        edits.push(TextEdit {
            range: self.range,
            new_text: function_call,
        });

        // Edit 2: Insert function above current location
        // Find the start of the line to insert before
        let insert_position = Position {
            line: self.range.start.line.saturating_sub(2),
            character: 0,
        };
        edits.push(TextEdit {
            range: Range {
                start: insert_position,
                end: insert_position,
            },
            new_text: format!("{}\n\n", new_function),
        });

        // Step 6: Create workspace edit
        let mut changes = std::collections::HashMap::new();
        changes.insert(self.uri.clone(), edits);

        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Analyze the scope of the selection
    fn analyze_scope(&self) -> Result<ScopeAnalysis, String> {
        // TODO: Parse the file and analyze scope
        // For now, return empty analysis
        Ok(ScopeAnalysis {
            parameters: vec![],
            return_values: vec![],
            local_variables: vec![],
            captured: vec![],
            has_early_return: false,
            has_control_flow: false,
        })
    }

    /// Generate the new function
    fn generate_function(&self, name: &str, analysis: &ScopeAnalysis, body: &str) -> String {
        // Convert scope analysis to parameters
        let parameters: Vec<Parameter> = analysis
            .parameters
            .iter()
            .map(|var| Parameter {
                name: var.name.clone(),
                pattern: None,
                type_: windjammer::parser::Type::Custom(
                    var.type_name
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                ownership: windjammer::parser::OwnershipHint::Inferred,
            })
            .collect();

        // Determine return type
        let return_type = if analysis.return_values.is_empty() {
            None
        } else if analysis.return_values.len() == 1 {
            Some(windjammer::parser::Type::Custom(
                analysis.return_values[0]
                    .type_name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            ))
        } else {
            // Multiple return values → tuple
            let types: Vec<windjammer::parser::Type> = analysis
                .return_values
                .iter()
                .map(|var| {
                    windjammer::parser::Type::Custom(
                        var.type_name
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                    )
                })
                .collect();
            Some(windjammer::parser::Type::Tuple(types))
        };

        ast_utils::generate_function(name, &parameters, &return_type, body, 0)
    }

    /// Generate the function call to replace the selection
    fn generate_call(&self, name: &str, analysis: &ScopeAnalysis) -> String {
        let args: Vec<String> = analysis
            .parameters
            .iter()
            .map(|var| var.name.clone())
            .collect();

        let call = ast_utils::generate_function_call(name, &args);

        // Handle return values
        if analysis.return_values.is_empty() {
            call
        } else if analysis.return_values.len() == 1 {
            format!("let {} = {}", analysis.return_values[0].name, call)
        } else {
            // Multiple returns → tuple destructuring
            let vars = analysis
                .return_values
                .iter()
                .map(|v| v.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("let ({}) = {}", vars, call)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_function_creation() {
        // Basic smoke test - will expand as we implement
    }
}
