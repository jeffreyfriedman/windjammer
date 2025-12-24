/// Add `mut` keyword quick-fix
///
/// Provides a one-click fix for mutability errors by adding the `mut` keyword
/// to variable declarations.
use tower_lsp::lsp_types::*;

pub struct AddMutFix {
    uri: Url,
    variable_name: String,
    line: u32,
}

impl AddMutFix {
    pub fn new(uri: Url, variable_name: String, line: u32) -> Self {
        Self {
            uri,
            variable_name,
            line,
        }
    }

    /// Create a code action for adding `mut` keyword
    pub fn create_action(&self) -> CodeAction {
        CodeAction {
            title: format!("Add `mut` to `{}`", self.variable_name),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: None, // Will be computed when diagnostics are available
            command: Some(Command {
                title: format!("Add `mut` to `{}`", self.variable_name),
                command: "windjammer.quickfix.addMut".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(&self.uri).unwrap(),
                    serde_json::to_value(&self.variable_name).unwrap(),
                    serde_json::to_value(self.line).unwrap(),
                ]),
            }),
            is_preferred: Some(true), // Suggest this as the primary fix
            disabled: None,
            data: None,
        }
    }

    /// Execute the fix by generating a workspace edit
    pub fn execute(&self, source: &str) -> Result<WorkspaceEdit, String> {
        let lines: Vec<&str> = source.lines().collect();

        if self.line as usize >= lines.len() {
            return Err("Line number out of range".to_string());
        }

        let line_text = lines[self.line as usize];

        // Find "let variable_name" and replace with "let mut variable_name"
        let pattern = format!("let {}", self.variable_name);
        let replacement = format!("let mut {}", self.variable_name);

        if let Some(col) = line_text.find(&pattern) {
            let start = Position {
                line: self.line,
                character: col as u32,
            };
            let end = Position {
                line: self.line,
                character: (col + pattern.len()) as u32,
            };

            let text_edit = TextEdit {
                range: Range { start, end },
                new_text: replacement,
            };

            let mut changes = std::collections::HashMap::new();
            changes.insert(self.uri.clone(), vec![text_edit]);

            Ok(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            })
        } else {
            Err(format!(
                "Could not find 'let {}' on line {}",
                self.variable_name, self.line
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_mut_fix() {
        let uri = Url::parse("file:///test.wj").unwrap();
        let fix = AddMutFix::new(uri.clone(), "x".to_string(), 1);

        let source = r#"fn main() {
    let x = 10
    x = 20
}
"#;

        let edit = fix.execute(source).expect("Should create edit");

        let changes = edit.changes.expect("Should have changes");
        let edits = changes.get(&uri).expect("Should have edits for URI");

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "let mut x");
    }

    #[test]
    fn test_add_mut_action() {
        let uri = Url::parse("file:///test.wj").unwrap();
        let fix = AddMutFix::new(uri.clone(), "count".to_string(), 2);

        let action = fix.create_action();

        assert_eq!(action.title, "Add `mut` to `count`");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert_eq!(action.is_preferred, Some(true));
    }
}
