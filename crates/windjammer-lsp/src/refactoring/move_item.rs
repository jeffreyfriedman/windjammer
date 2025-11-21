#![allow(dead_code)] // Refactoring implementation - some parts planned for future versions
//! Move Item refactoring
//!
//! Move functions, structs, and other items between files while
//! automatically updating imports.

use super::ast_utils;
use crate::database::WindjammerDatabase;
use tower_lsp::lsp_types::*;

/// Move an item (function, struct, etc.) to another file
pub struct MoveItem<'a> {
    db: &'a WindjammerDatabase,
    source_uri: Url,
    target_uri: Url,
    position: Position,
}

/// Type of item being moved
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemType {
    Function,
    Struct,
    Enum,
    Trait,
    Const,
    Static,
}

/// Result of analyzing an item for moving
#[derive(Debug, Clone)]
pub struct MoveAnalysis {
    /// Type of item
    pub item_type: ItemType,
    /// Name of the item
    pub item_name: String,
    /// Full text of the item
    pub item_text: String,
    /// Range of the item in source file
    pub item_range: Range,
    /// Items that depend on this item (in source file)
    pub dependencies: Vec<String>,
    /// Whether it's safe to move
    pub is_safe: bool,
    /// Reason if not safe
    pub unsafe_reason: Option<String>,
}

impl<'a> MoveItem<'a> {
    /// Create a new move item refactoring
    pub fn new(
        db: &'a WindjammerDatabase,
        source_uri: Url,
        target_uri: Url,
        position: Position,
    ) -> Self {
        Self {
            db,
            source_uri,
            target_uri,
            position,
        }
    }

    /// Execute the refactoring
    pub fn execute(
        &self,
        source_content: &str,
        target_content: &str,
    ) -> Result<WorkspaceEdit, String> {
        // Step 1: Analyze the item at the cursor
        let analysis = self.analyze_item(source_content)?;

        // Step 2: Safety checks
        if !analysis.is_safe {
            return Err(analysis
                .unsafe_reason
                .unwrap_or_else(|| "Cannot move item: unsafe".to_string()));
        }

        // Step 3: Check for circular dependencies
        let target_module = self.extract_module_name(&self.target_uri);
        if self.would_create_cycle(&analysis, source_content, &target_module) {
            return Err(format!(
                "Cannot move {}: would create circular dependency",
                analysis.item_name
            ));
        }

        // Step 4: Create text edits
        let mut changes = std::collections::HashMap::new();

        // Prepare source file edits
        let mut source_edits = vec![];

        // Remove item from source file
        source_edits.push(TextEdit {
            range: analysis.item_range,
            new_text: String::new(), // Delete the item
        });

        // Add import in source file if there are usages
        let usages = self.find_item_usages(source_content, &analysis.item_name);
        if !usages.is_empty() {
            let import_edit =
                self.create_import_edit(source_content, &analysis.item_name, &target_module);
            if let Some(edit) = import_edit {
                source_edits.push(edit);
            }
        }

        // Prepare target file edits
        let mut target_edits = vec![];

        // Add to target file (append at end)
        let target_position = self.find_insert_position(target_content);
        target_edits.push(TextEdit {
            range: Range {
                start: target_position,
                end: target_position,
            },
            new_text: format!("\n{}\n", analysis.item_text),
        });

        changes.insert(self.source_uri.clone(), source_edits);
        changes.insert(self.target_uri.clone(), target_edits);

        // Step 5: Create workspace edit
        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Analyze the item at the cursor position
    fn analyze_item(&self, source: &str) -> Result<MoveAnalysis, String> {
        // Find the item definition at the cursor
        let (item_type, item_name, item_range, item_text) = self.find_item_at_cursor(source)?;

        // Track dependencies
        let dependencies = self.track_dependencies(source, &item_text);

        // Check if it's safe to move
        let (is_safe, unsafe_reason) = self.check_safety(&item_name, &dependencies);

        Ok(MoveAnalysis {
            item_type,
            item_name,
            item_text,
            item_range,
            dependencies,
            is_safe,
            unsafe_reason,
        })
    }

    /// Find the item definition at the cursor
    fn find_item_at_cursor(
        &self,
        source: &str,
    ) -> Result<(ItemType, String, Range, String), String> {
        let cursor_byte = ast_utils::position_to_byte_offset(source, self.position);

        // Try to find different types of items
        // Pattern for functions: fn name(...) { ... }
        if let Ok(result) = self.find_function(source, cursor_byte) {
            return Ok(result);
        }

        // Pattern for structs: struct Name { ... }
        if let Ok(result) = self.find_struct(source, cursor_byte) {
            return Ok(result);
        }

        // Pattern for enums: enum Name { ... }
        if let Ok(result) = self.find_enum(source, cursor_byte) {
            return Ok(result);
        }

        Err("No movable item found at cursor".to_string())
    }

    /// Find a function at the cursor
    fn find_function(
        &self,
        source: &str,
        cursor_byte: usize,
    ) -> Result<(ItemType, String, Range, String), String> {
        // Pattern: fn name(...) { ... }
        // Simplified: just find fn keyword and capture until the end of the block
        let lines: Vec<&str> = source.lines().collect();
        let cursor_line = ast_utils::byte_offset_to_position(source, cursor_byte).line as usize;

        if cursor_line >= lines.len() {
            return Err("Cursor out of bounds".to_string());
        }

        // Find the function definition line
        let mut start_line = cursor_line;
        while start_line > 0 && !lines[start_line].trim_start().starts_with("fn ") {
            start_line -= 1;
        }

        if !lines[start_line].trim_start().starts_with("fn ") {
            return Err("No function found".to_string());
        }

        // Extract function name
        let fn_line = lines[start_line];
        let name_start = fn_line.find("fn ").ok_or("No fn keyword")? + 3;
        let name_end = fn_line[name_start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| name_start + i)
            .unwrap_or(fn_line.len());
        let function_name = fn_line[name_start..name_end].to_string();

        // Find the end of the function (closing brace)
        let mut end_line = start_line;
        let mut brace_count = 0;
        let mut found_opening = false;

        for (i, line) in lines.iter().enumerate().skip(start_line) {
            for ch in line.chars() {
                if ch == '{' {
                    brace_count += 1;
                    found_opening = true;
                } else if ch == '}' {
                    brace_count -= 1;
                }
            }

            if found_opening && brace_count == 0 {
                end_line = i;
                break;
            }
        }

        // Extract the full function text
        let item_text = lines[start_line..=end_line].join("\n");

        let start_pos = Position {
            line: start_line as u32,
            character: 0,
        };
        let end_pos = Position {
            line: (end_line + 1) as u32,
            character: 0,
        };

        Ok((
            ItemType::Function,
            function_name,
            Range {
                start: start_pos,
                end: end_pos,
            },
            item_text,
        ))
    }

    /// Find a struct at the cursor
    fn find_struct(
        &self,
        source: &str,
        cursor_byte: usize,
    ) -> Result<(ItemType, String, Range, String), String> {
        let lines: Vec<&str> = source.lines().collect();
        let cursor_line = ast_utils::byte_offset_to_position(source, cursor_byte).line as usize;

        if cursor_line >= lines.len() {
            return Err("Cursor out of bounds".to_string());
        }

        // Find the struct definition line
        let mut start_line = cursor_line;
        while start_line > 0 && !lines[start_line].trim_start().starts_with("struct ") {
            start_line -= 1;
        }

        if !lines[start_line].trim_start().starts_with("struct ") {
            return Err("No struct found".to_string());
        }

        // Extract struct name
        let struct_line = lines[start_line];
        let name_start = struct_line.find("struct ").ok_or("No struct keyword")? + 7;
        let name_end = struct_line[name_start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| name_start + i)
            .unwrap_or(struct_line.len());
        let struct_name = struct_line[name_start..name_end].to_string();

        // Find the end (closing brace or semicolon)
        let mut end_line = start_line;
        if struct_line.contains('{') {
            // Struct with fields
            let mut brace_count = 0;
            let mut found_opening = false;

            for (i, line) in lines.iter().enumerate().skip(start_line) {
                for ch in line.chars() {
                    if ch == '{' {
                        brace_count += 1;
                        found_opening = true;
                    } else if ch == '}' {
                        brace_count -= 1;
                    }
                }

                if found_opening && brace_count == 0 {
                    end_line = i;
                    break;
                }
            }
        } else {
            // Tuple struct or unit struct (ends with semicolon)
            end_line = start_line;
        }

        let item_text = lines[start_line..=end_line].join("\n");

        let start_pos = Position {
            line: start_line as u32,
            character: 0,
        };
        let end_pos = Position {
            line: (end_line + 1) as u32,
            character: 0,
        };

        Ok((
            ItemType::Struct,
            struct_name,
            Range {
                start: start_pos,
                end: end_pos,
            },
            item_text,
        ))
    }

    /// Find an enum at the cursor
    fn find_enum(
        &self,
        _source: &str,
        _cursor_byte: usize,
    ) -> Result<(ItemType, String, Range, String), String> {
        // Simplified version - similar to struct
        Err("Enum finding not implemented yet".to_string())
    }

    /// Find where to insert the item in the target file
    fn find_insert_position(&self, target_content: &str) -> Position {
        // Insert at the end of the file
        let lines = target_content.lines().count();
        Position {
            line: lines as u32,
            character: 0,
        }
    }

    /// Check if it's safe to move the item
    fn check_safety(&self, item_name: &str, dependencies: &[String]) -> (bool, Option<String>) {
        // Check if item has too many dependencies
        if dependencies.len() > 10 {
            return (
                false,
                Some(format!(
                    "{} has {} dependencies - consider refactoring first",
                    item_name,
                    dependencies.len()
                )),
            );
        }

        (true, None)
    }

    /// Extract module name from URI
    fn extract_module_name(&self, uri: &Url) -> String {
        uri.path()
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .trim_end_matches(".wj")
            .to_string()
    }

    /// Find usages of an item in source code
    fn find_item_usages(&self, source: &str, item_name: &str) -> Vec<Range> {
        let mut usages = vec![];
        let pattern = format!(r"\b{}\b", regex::escape(item_name));
        let re = regex::Regex::new(&pattern).unwrap();

        for (line_num, line) in source.lines().enumerate() {
            for cap in re.find_iter(line) {
                let start_pos = Position {
                    line: line_num as u32,
                    character: cap.start() as u32,
                };
                let end_pos = Position {
                    line: line_num as u32,
                    character: cap.end() as u32,
                };
                usages.push(Range {
                    start: start_pos,
                    end: end_pos,
                });
            }
        }

        usages
    }

    /// Create an import edit for the moved item
    fn create_import_edit(
        &self,
        source: &str,
        item_name: &str,
        target_module: &str,
    ) -> Option<TextEdit> {
        // Find if there's already a use statement section
        let lines: Vec<&str> = source.lines().collect();
        let mut insert_line = 0;

        // Find the last use statement
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("use ") {
                insert_line = i + 1;
            }
        }

        // If no use statements, insert after any initial comments
        if insert_line == 0 {
            for (i, line) in lines.iter().enumerate() {
                if !line.trim().is_empty() && !line.trim().starts_with("//") {
                    insert_line = i;
                    break;
                }
            }
        }

        let import_statement = format!("use {}.{}\n", target_module, item_name);

        Some(TextEdit {
            range: Range {
                start: Position {
                    line: insert_line as u32,
                    character: 0,
                },
                end: Position {
                    line: insert_line as u32,
                    character: 0,
                },
            },
            new_text: import_statement,
        })
    }

    /// Check if moving would create a circular dependency
    fn would_create_cycle(
        &self,
        analysis: &MoveAnalysis,
        source_content: &str,
        _target_module: &str,
    ) -> bool {
        // Check if target module imports from source module
        let source_module = self.extract_module_name(&self.source_uri);

        // Look for use statements in target that reference source module
        let pattern = format!(r"use\s+{}\.(\w+)", regex::escape(&source_module));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if re.is_match(source_content) {
                // Target imports from source, check if moved item is in those imports
                for cap in re.captures_iter(source_content) {
                    if let Some(imported_item) = cap.get(1) {
                        // Check if the moved item depends on this imported item
                        if analysis
                            .dependencies
                            .contains(&imported_item.as_str().to_string())
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Track dependencies of an item
    fn track_dependencies(&self, source: &str, item_text: &str) -> Vec<String> {
        let mut dependencies = vec![];

        // Find all identifiers in the item that might be dependencies
        // Pattern: word boundaries, excluding keywords
        let keywords = vec![
            "fn", "struct", "enum", "let", "mut", "if", "else", "for", "while", "return",
        ];

        let re = regex::Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\b").unwrap();

        for cap in re.find_iter(item_text) {
            let word = cap.as_str();

            // Skip keywords
            if keywords.contains(&word) {
                continue;
            }

            // Check if this identifier is defined elsewhere in source
            if self.is_external_dependency(source, item_text, word)
                && !dependencies.contains(&word.to_string())
            {
                dependencies.push(word.to_string());
            }
        }

        dependencies
    }

    /// Check if an identifier is an external dependency
    fn is_external_dependency(&self, source: &str, item_text: &str, identifier: &str) -> bool {
        // Check if identifier is defined in source but outside the item
        let pattern = format!(
            r"\b(fn|struct|enum|const|static)\s+{}\b",
            regex::escape(identifier)
        );

        if let Ok(re) = regex::Regex::new(&pattern) {
            for cap in re.find_iter(source) {
                let match_text = cap.as_str();
                // If found in source but not in item text, it's an external dependency
                if !item_text.contains(match_text) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_function() {
        let db = WindjammerDatabase::new();
        let source_uri = Url::parse("file:///source.wj").unwrap();
        let target_uri = Url::parse("file:///target.wj").unwrap();
        let position = Position {
            line: 0,
            character: 3,
        };

        let mover = MoveItem::new(&db, source_uri, target_uri, position);

        let source = r#"fn calculate(x: int) -> int {
    x * 2
}
"#;

        let cursor_byte = ast_utils::position_to_byte_offset(source, position);
        let result = mover.find_function(source, cursor_byte);

        assert!(result.is_ok(), "Should find function");
        let (item_type, name, _, _) = result.unwrap();
        assert_eq!(item_type, ItemType::Function);
        assert_eq!(name, "calculate");
    }

    #[test]
    fn test_find_struct() {
        let db = WindjammerDatabase::new();
        let source_uri = Url::parse("file:///source.wj").unwrap();
        let target_uri = Url::parse("file:///target.wj").unwrap();
        let position = Position {
            line: 0,
            character: 7,
        };

        let mover = MoveItem::new(&db, source_uri, target_uri, position);

        let source = r#"struct User {
    name: string,
    age: int,
}
"#;

        let cursor_byte = ast_utils::position_to_byte_offset(source, position);
        let result = mover.find_struct(source, cursor_byte);

        assert!(result.is_ok(), "Should find struct");
        let (item_type, name, _, _) = result.unwrap();
        assert_eq!(item_type, ItemType::Struct);
        assert_eq!(name, "User");
    }
}
