//! Apply LSP workspace edits to source text (shared by LSP and MCP).

use tower_lsp::lsp_types::WorkspaceEdit;

use super::ast_utils;

/// Apply a single-file `WorkspaceEdit` to `source`, returning the refactored text.
pub fn apply_workspace_edit(source: &str, edit: &WorkspaceEdit) -> Result<String, String> {
    let changes = edit
        .changes
        .as_ref()
        .ok_or_else(|| "Workspace edit has no text changes".to_string())?;

    let edits = changes
        .values()
        .next()
        .ok_or_else(|| "Workspace edit changes map is empty".to_string())?;

    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by(|a, b| {
        b.range
            .start
            .line
            .cmp(&a.range.start.line)
            .then(b.range.start.character.cmp(&a.range.start.character))
    });

    let mut result = source.to_string();
    for text_edit in sorted {
        let (start, end) = ast_utils::range_to_offsets(&result, text_edit.range)
            .ok_or_else(|| "Text edit range out of bounds".to_string())?;
        if start > result.len() || end > result.len() || start > end {
            return Err("Text edit range out of bounds".to_string());
        }
        result.replace_range(start..end, &text_edit.new_text);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tower_lsp::lsp_types::{Range, TextEdit, Url, WorkspaceEdit};

    #[test]
    fn apply_single_replacement() {
        let source = "let x = 1\nlet y = 2\n";
        let mut changes = HashMap::new();
        changes.insert(
            Url::parse("file:///test.wj").unwrap(),
            vec![TextEdit {
                range: Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 8,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 9,
                    },
                },
                new_text: "42".to_string(),
            }],
        );
        let edit = WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };
        let out = apply_workspace_edit(source, &edit).unwrap();
        assert!(out.contains("let x = 42"));
    }
}
