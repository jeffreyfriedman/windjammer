//! Code refactoring and transformation capabilities
//!
//! This module provides advanced refactoring tools that make it easy to safely
//! transform code while maintaining correctness. All refactorings are:
//! - Type-aware: Understand the code structure and types
//! - Safe: Won't break existing code
//! - Preview-able: Show changes before applying
//! - Undo-able: Via LSP workspace edits

pub mod ast_utils;
pub mod extract_function;
pub mod inline;
pub mod introduce_variable;
pub mod scope_analyzer;

use crate::database::WindjammerDatabase;
use tower_lsp::lsp_types::*;

/// Main refactoring engine that coordinates all refactoring operations
pub struct RefactoringEngine<'a> {
    db: &'a WindjammerDatabase,
}

impl<'a> RefactoringEngine<'a> {
    /// Create a new refactoring engine
    pub fn new(db: &'a WindjammerDatabase) -> Self {
        Self { db }
    }

    /// Get the database
    pub fn db(&self) -> &WindjammerDatabase {
        self.db
    }

    /// Generate code actions for a given text position
    pub fn code_actions(
        &self,
        uri: &Url,
        range: Range,
        _context: &CodeActionContext,
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Extract Function - only show if selection is non-empty
        if range.start != range.end {
            if let Some(action) = self.extract_function_action(uri, range) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        // Inline Variable - show if cursor is on a variable definition
        if let Some(action) = self.inline_variable_action(uri, range.start) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        // Introduce Variable - show if selection is an expression
        if range.start != range.end {
            if let Some(action) = self.introduce_variable_action(uri, range) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        actions
    }

    /// Create "Extract Function" code action
    fn extract_function_action(&self, uri: &Url, range: Range) -> Option<CodeAction> {
        // TODO: Check if selection is valid for extraction
        // For now, always offer the action if there's a selection

        Some(CodeAction {
            title: "Extract Function".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: None, // Will be computed when user accepts
            command: Some(Command {
                title: "Extract Function".to_string(),
                command: "windjammer.refactor.extractFunction".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(uri).ok()?,
                    serde_json::to_value(range).ok()?,
                ]),
            }),
            is_preferred: Some(true),
            disabled: None,
            data: None,
        })
    }

    /// Create "Inline Variable" code action
    fn inline_variable_action(&self, _uri: &Url, _position: Position) -> Option<CodeAction> {
        // TODO: Check if position is on a variable definition
        // For now, return None until implemented
        None
    }

    /// Create "Introduce Variable" code action
    fn introduce_variable_action(&self, _uri: &Url, _range: Range) -> Option<CodeAction> {
        // TODO: Check if selection is an expression
        // For now, return None until implemented
        None
    }

    /// Execute "Extract Function" refactoring
    pub fn execute_extract_function(
        &self,
        uri: &Url,
        range: Range,
        function_name: &str,
        source: &str,
    ) -> Result<WorkspaceEdit, String> {
        let extractor = extract_function::ExtractFunction::new(self.db, uri.clone(), range);
        extractor.execute(function_name, source)
    }

    /// Execute "Inline Variable" refactoring
    pub fn execute_inline_variable(
        &self,
        uri: &Url,
        position: Position,
        source: &str,
    ) -> Result<WorkspaceEdit, String> {
        let inliner = inline::InlineRefactoring::new(self.db, uri.clone(), position);
        inliner.execute(source)
    }
}

/// Result of a refactoring operation
#[derive(Debug, Clone)]
pub struct RefactoringResult {
    /// The workspace edit to apply
    pub edit: WorkspaceEdit,
    /// Preview text to show the user
    pub preview: String,
    /// Success message
    pub message: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_refactoring_engine_creation() {
        // Basic smoke test - will expand as we implement features
        // For now, just verify the module compiles
    }
}
