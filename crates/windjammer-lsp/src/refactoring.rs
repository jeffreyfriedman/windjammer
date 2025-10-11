use tower_lsp::lsp_types::*;
use windjammer::parser::{FunctionDecl, Item, Program, Statement};

/// Provides refactoring code actions for Windjammer code
pub struct RefactoringProvider {
    program: Option<Program>,
}

impl RefactoringProvider {
    pub fn new() -> Self {
        Self { program: None }
    }

    pub fn update_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    /// Get all available code actions for the given range
    pub fn get_code_actions(
        &self,
        _uri: &Url,
        range: Range,
        _source: &str,
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Extract Function: Available when a range is selected
        if range.start != range.end {
            actions.push(self.create_extract_function_action(range));
        }

        // Inline Variable: Available when cursor is on a let binding
        if let Some(ref program) = self.program {
            if let Some(action) = self.create_inline_variable_action(range, program) {
                actions.push(action);
            }
        }

        // Convert to method: Available when cursor is on a function call
        if let Some(ref program) = self.program {
            if let Some(action) = self.create_convert_to_method_action(range, program) {
                actions.push(action);
            }
        }

        actions
    }

    /// Create "Extract Function" code action
    fn create_extract_function_action(&self, range: Range) -> CodeActionOrCommand {
        CodeActionOrCommand::CodeAction(CodeAction {
            title: "Extract Function".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: None, // Will be computed on-demand
            command: Some(Command {
                title: "Extract Function".to_string(),
                command: "windjammer.refactor.extractFunction".to_string(),
                arguments: Some(vec![serde_json::json!({
                    "range": range,
                })]),
            }),
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    /// Create "Inline Variable" code action
    fn create_inline_variable_action(
        &self,
        range: Range,
        program: &Program,
    ) -> Option<CodeActionOrCommand> {
        // Check if cursor is on a let binding
        let var_name = self.find_variable_at_position(range.start, program)?;

        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Inline variable '{}'", var_name),
            kind: Some(CodeActionKind::REFACTOR_INLINE),
            diagnostics: None,
            edit: None,
            command: Some(Command {
                title: "Inline Variable".to_string(),
                command: "windjammer.refactor.inlineVariable".to_string(),
                arguments: Some(vec![serde_json::json!({
                    "variableName": var_name,
                    "position": range.start,
                })]),
            }),
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }))
    }

    /// Create "Convert to Method" code action
    fn create_convert_to_method_action(
        &self,
        range: Range,
        _program: &Program,
    ) -> Option<CodeActionOrCommand> {
        // This is a placeholder for converting function calls to method calls
        // e.g., `len(vec)` -> `vec.len()`
        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Convert to method call".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            diagnostics: None,
            edit: None,
            command: Some(Command {
                title: "Convert to Method".to_string(),
                command: "windjammer.refactor.convertToMethod".to_string(),
                arguments: Some(vec![serde_json::json!({
                    "position": range.start,
                })]),
            }),
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }))
    }

    /// Find a variable declaration at the given position
    fn find_variable_at_position(&self, position: Position, program: &Program) -> Option<String> {
        for item in &program.items {
            if let Item::Function(func) = item {
                if let Some(var_name) = self.find_variable_in_function(position, func) {
                    return Some(var_name);
                }
            }
        }
        None
    }

    fn find_variable_in_function(&self, position: Position, func: &FunctionDecl) -> Option<String> {
        for stmt in &func.body {
            if let Some(var_name) = self.find_variable_in_statement(position, stmt) {
                return Some(var_name);
            }
        }
        None
    }

    fn find_variable_in_statement(&self, position: Position, stmt: &Statement) -> Option<String> {
        match stmt {
            Statement::Let { name, .. } => {
                // Check if position is on this line (simplified)
                // In a real implementation, we'd use source spans
                Some(name.clone())
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for s in then_block {
                    if let Some(name) = self.find_variable_in_statement(position, s) {
                        return Some(name);
                    }
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        if let Some(name) = self.find_variable_in_statement(position, s) {
                            return Some(name);
                        }
                    }
                }
                None
            }
            Statement::For { body, .. } | Statement::While { body, .. } => {
                for s in body {
                    if let Some(name) = self.find_variable_in_statement(position, s) {
                        return Some(name);
                    }
                }
                None
            }
            Statement::Loop { body } => {
                for s in body {
                    if let Some(name) = self.find_variable_in_statement(position, s) {
                        return Some(name);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Compute the actual edit for extracting a function
    pub fn compute_extract_function_edit(
        &self,
        range: Range,
        source: &str,
    ) -> Option<WorkspaceEdit> {
        // Extract the selected text
        let selected_text = self.extract_text_from_range(source, range)?;

        // Generate a new function name
        let function_name = "extracted_function";

        // Create the new function
        let new_function = format!(
            "fn {}() {{\n    {}\n}}\n\n",
            function_name,
            selected_text.trim()
        );

        // Create the function call to replace the selection
        let function_call = format!("{}()", function_name);

        // Build the workspace edit
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            Url::parse("file:///dummy").unwrap(), // Will be replaced with actual URI
            vec![
                // Insert new function at the top
                TextEdit {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    new_text: new_function,
                },
                // Replace selected text with function call
                TextEdit {
                    range,
                    new_text: function_call,
                },
            ],
        );

        Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Compute the actual edit for inlining a variable
    pub fn compute_inline_variable_edit(
        &self,
        var_name: &str,
        _source: &str,
        program: &Program,
    ) -> Option<WorkspaceEdit> {
        // Find the variable definition and its initializer
        let (def_range, initializer) = self.find_variable_definition(var_name, program)?;

        // Find all usages of the variable
        let usages = self.find_variable_usages(var_name, program);

        // Build edits: remove definition, replace all usages
        let mut edits = Vec::new();

        // Remove the let statement
        edits.push(TextEdit {
            range: def_range,
            new_text: String::new(),
        });

        // Replace all usages with the initializer
        for usage_range in usages {
            edits.push(TextEdit {
                range: usage_range,
                new_text: initializer.clone(),
            });
        }

        let mut changes = std::collections::HashMap::new();
        changes.insert(Url::parse("file:///dummy").unwrap(), edits);

        Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    fn extract_text_from_range(&self, source: &str, range: Range) -> Option<String> {
        let lines: Vec<&str> = source.lines().collect();
        let start_line = range.start.line as usize;
        let end_line = range.end.line as usize;

        if start_line >= lines.len() || end_line >= lines.len() {
            return None;
        }

        if start_line == end_line {
            let line = lines[start_line];
            let start_char = range.start.character as usize;
            let end_char = range.end.character as usize;
            Some(line.get(start_char..end_char)?.to_string())
        } else {
            let mut result = String::new();
            for (i, line) in lines
                .iter()
                .enumerate()
                .skip(start_line)
                .take(end_line - start_line + 1)
            {
                if i == start_line {
                    result.push_str(&line[range.start.character as usize..]);
                } else if i == end_line {
                    result.push_str(&line[..range.end.character as usize]);
                } else {
                    result.push_str(line);
                }
                if i < end_line {
                    result.push('\n');
                }
            }
            Some(result)
        }
    }

    fn find_variable_definition(
        &self,
        _var_name: &str,
        _program: &Program,
    ) -> Option<(Range, String)> {
        // Placeholder: would need source spans in AST
        None
    }

    fn find_variable_usages(&self, _var_name: &str, _program: &Program) -> Vec<Range> {
        // Placeholder: would need source spans in AST
        Vec::new()
    }
}
