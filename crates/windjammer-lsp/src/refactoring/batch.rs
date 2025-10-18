//! Batch refactorings
//!
//! Apply multiple refactorings at once with conflict resolution.
#![allow(dead_code)] // Planned feature for batch refactoring operations

use super::RefactoringEngine;
use tower_lsp::lsp_types::*;

/// A batch of refactorings to apply together
pub struct BatchRefactoring<'a> {
    engine: &'a RefactoringEngine<'a>,
    operations: Vec<RefactoringOperation>,
}

/// A single refactoring operation in a batch
#[derive(Debug, Clone)]
pub enum RefactoringOperation {
    ExtractFunction {
        uri: Url,
        range: Range,
        function_name: String,
        source: String,
    },
    InlineVariable {
        uri: Url,
        position: Position,
        source: String,
    },
    IntroduceVariable {
        uri: Url,
        range: Range,
        variable_name: String,
        source: String,
    },
    ChangeSignature {
        uri: Url,
        position: Position,
        changes: Vec<super::change_signature::ParameterChange>,
        source: String,
    },
    MoveItem {
        source_uri: Url,
        target_uri: Url,
        position: Position,
        source_content: String,
        target_content: String,
    },
}

/// Result of applying a batch of refactorings
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Combined workspace edit
    pub workspace_edit: WorkspaceEdit,
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

impl<'a> BatchRefactoring<'a> {
    /// Create a new batch refactoring
    pub fn new(engine: &'a RefactoringEngine<'a>) -> Self {
        Self {
            engine,
            operations: vec![],
        }
    }

    /// Add an operation to the batch
    pub fn add_operation(&mut self, operation: RefactoringOperation) -> &mut Self {
        self.operations.push(operation);
        self
    }

    /// Execute all refactorings in the batch
    pub fn execute(&self) -> Result<BatchResult, String> {
        let mut combined_changes = std::collections::HashMap::new();
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = vec![];
        let mut warnings = vec![];

        // Check for conflicts before applying
        if let Some(conflict) = self.detect_conflicts() {
            warnings.push(format!("Conflict detected: {}", conflict));
        }

        // Apply each operation
        for (i, operation) in self.operations.iter().enumerate() {
            match self.apply_operation(operation) {
                Ok(workspace_edit) => {
                    successful += 1;

                    // Merge changes
                    if let Some(changes) = workspace_edit.changes {
                        for (uri, edits) in changes {
                            combined_changes
                                .entry(uri)
                                .or_insert_with(Vec::new)
                                .extend(edits);
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("Operation {}: {}", i + 1, e));
                }
            }
        }

        // Check if we should proceed
        if failed > 0 && successful == 0 {
            return Err(format!(
                "All {} operations failed: {}",
                failed,
                errors.join("; ")
            ));
        }

        Ok(BatchResult {
            workspace_edit: WorkspaceEdit {
                changes: Some(combined_changes),
                document_changes: None,
                change_annotations: None,
            },
            successful,
            failed,
            errors,
            warnings,
        })
    }

    /// Apply a single operation
    fn apply_operation(&self, operation: &RefactoringOperation) -> Result<WorkspaceEdit, String> {
        match operation {
            RefactoringOperation::ExtractFunction {
                uri,
                range,
                function_name,
                source,
            } => self
                .engine
                .execute_extract_function(uri, *range, function_name, source),

            RefactoringOperation::InlineVariable {
                uri,
                position,
                source,
            } => self.engine.execute_inline_variable(uri, *position, source),

            RefactoringOperation::IntroduceVariable {
                uri,
                range,
                variable_name,
                source,
            } => self
                .engine
                .execute_introduce_variable(uri, *range, variable_name, source),

            RefactoringOperation::ChangeSignature {
                uri,
                position,
                changes,
                source,
            } => self
                .engine
                .execute_change_signature(uri, *position, changes, source),

            RefactoringOperation::MoveItem {
                source_uri,
                target_uri,
                position,
                source_content,
                target_content,
            } => self.engine.execute_move_item(
                source_uri,
                target_uri,
                *position,
                source_content,
                target_content,
            ),
        }
    }

    /// Detect conflicts between operations
    fn detect_conflicts(&self) -> Option<String> {
        // Check for overlapping ranges in the same file
        let mut file_ranges: std::collections::HashMap<String, Vec<Range>> =
            std::collections::HashMap::new();

        for operation in &self.operations {
            let (uri, range_opt) = match operation {
                RefactoringOperation::ExtractFunction { uri, range, .. } => {
                    (uri.to_string(), Some(*range))
                }
                RefactoringOperation::InlineVariable { uri, position, .. } => (
                    uri.to_string(),
                    Some(Range {
                        start: *position,
                        end: *position,
                    }),
                ),
                RefactoringOperation::IntroduceVariable { uri, range, .. } => {
                    (uri.to_string(), Some(*range))
                }
                RefactoringOperation::ChangeSignature { uri, position, .. } => (
                    uri.to_string(),
                    Some(Range {
                        start: *position,
                        end: *position,
                    }),
                ),
                RefactoringOperation::MoveItem { source_uri, .. } => (source_uri.to_string(), None),
            };

            if let Some(range) = range_opt {
                file_ranges.entry(uri).or_default().push(range);
            }
        }

        // Check for overlaps
        for (uri, ranges) in file_ranges {
            for i in 0..ranges.len() {
                for j in (i + 1)..ranges.len() {
                    if Self::ranges_overlap(&ranges[i], &ranges[j]) {
                        return Some(format!(
                            "Overlapping operations in {}",
                            uri.split('/').next_back().unwrap_or("unknown")
                        ));
                    }
                }
            }
        }

        None
    }

    /// Check if two ranges overlap
    fn ranges_overlap(a: &Range, b: &Range) -> bool {
        // Check if ranges overlap
        !(a.end.line < b.start.line || b.end.line < a.start.line)
    }

    /// Get number of operations in the batch
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::WindjammerDatabase;

    #[test]
    fn test_batch_creation() {
        let db = WindjammerDatabase::new();
        let engine = RefactoringEngine::new(&db);
        let batch = BatchRefactoring::new(&engine);

        assert_eq!(batch.operation_count(), 0);
    }

    #[test]
    fn test_add_operation() {
        let db = WindjammerDatabase::new();
        let engine = RefactoringEngine::new(&db);
        let mut batch = BatchRefactoring::new(&engine);

        let uri = Url::parse("file:///test.wj").unwrap();
        batch.add_operation(RefactoringOperation::InlineVariable {
            uri,
            position: Position {
                line: 0,
                character: 0,
            },
            source: "let x = 1".to_string(),
        });

        assert_eq!(batch.operation_count(), 1);
    }

    #[test]
    fn test_conflict_detection() {
        let db = WindjammerDatabase::new();
        let engine = RefactoringEngine::new(&db);
        let mut batch = BatchRefactoring::new(&engine);

        let uri = Url::parse("file:///test.wj").unwrap();

        // Add two overlapping operations
        batch.add_operation(RefactoringOperation::ExtractFunction {
            uri: uri.clone(),
            range: Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
            },
            function_name: "helper".to_string(),
            source: String::new(),
        });

        batch.add_operation(RefactoringOperation::ExtractFunction {
            uri,
            range: Range {
                start: Position {
                    line: 3,
                    character: 0,
                },
                end: Position {
                    line: 7,
                    character: 0,
                },
            },
            function_name: "other".to_string(),
            source: String::new(),
        });

        let conflict = batch.detect_conflicts();
        assert!(conflict.is_some(), "Should detect overlapping operations");
    }
}
