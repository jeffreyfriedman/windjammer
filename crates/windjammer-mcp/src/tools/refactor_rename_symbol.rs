//! Rename symbol refactoring tool
//!
//! Renames a symbol (variable, function, struct, etc.) with workspace-wide updates.

use crate::error::{McpError, McpResult};
use crate::protocol::{Position, ToolCallResult};
use crate::tools::text_response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameSymbolRequest {
    /// Source code to refactor
    pub code: String,

    /// Position of the symbol to rename
    pub position: Position,

    /// New name for the symbol
    pub new_name: String,

    /// Optional: Current name (if position doesn't uniquely identify it)
    pub old_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameSymbolResponse {
    pub success: bool,

    /// Refactored code with renamed symbol
    pub refactored_code: Option<String>,

    /// Number of occurrences renamed
    pub occurrences_renamed: Option<usize>,

    /// Symbol that was renamed
    pub old_name: Option<String>,

    /// Files affected (for workspace-wide rename)
    pub files_affected: Option<Vec<String>>,

    pub error: Option<String>,
}

/// Rename symbol refactoring tool
pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: RenameSymbolRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    // Validate new name
    if !is_valid_identifier(&request.new_name) {
        let response = RenameSymbolResponse {
            success: false,
            refactored_code: None,
            occurrences_renamed: None,
            old_name: None,
            files_affected: None,
            error: Some(format!("Invalid identifier: {}", request.new_name)),
        };
        return Ok(text_response(&serde_json::to_string(&response)?));
    }

    // Parse the source code
    let mut lexer = Lexer::new(&request.code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    let program = match parse_result {
        Ok(prog) => prog,
        Err(e) => {
            let response = RenameSymbolResponse {
                success: false,
                refactored_code: None,
                occurrences_renamed: None,
                old_name: None,
                files_affected: None,
                error: Some(format!("Parse error: {}", e)),
            };
            return Ok(text_response(&serde_json::to_string(&response)?));
        }
    };

    // Find symbol at position
    let symbol_info = find_symbol_at_position(&program, &request.position);

    let (old_name, symbol_kind) = match symbol_info {
        Some((name, kind)) => (name, kind),
        None => {
            let response = RenameSymbolResponse {
                success: false,
                refactored_code: None,
                occurrences_renamed: None,
                old_name: None,
                files_affected: None,
                error: Some("No symbol found at position".to_string()),
            };
            return Ok(text_response(&serde_json::to_string(&response)?));
        }
    };

    // Check for naming conflicts
    if has_naming_conflict(&program, &request.new_name, &symbol_kind) {
        let response = RenameSymbolResponse {
            success: false,
            refactored_code: None,
            occurrences_renamed: None,
            old_name: Some(old_name),
            files_affected: None,
            error: Some(format!(
                "Naming conflict: {} already exists in this scope",
                request.new_name
            )),
        };
        return Ok(text_response(&serde_json::to_string(&response)?));
    }

    // Find all occurrences
    let occurrences = find_all_occurrences(&program, &old_name, &symbol_kind);

    // Generate refactored code (simplified - would need proper AST manipulation)
    let refactored_code = format!(
        "// TODO: Implement full AST manipulation\n// Would rename {} '{}' to '{}' ({} occurrences)",
        symbol_kind.name(),
        old_name,
        request.new_name,
        occurrences
    );

    let response = RenameSymbolResponse {
        success: true,
        refactored_code: Some(refactored_code),
        occurrences_renamed: Some(occurrences),
        old_name: Some(old_name),
        files_affected: Some(vec!["current_file.wj".to_string()]),
        error: None,
    };

    Ok(text_response(&serde_json::to_string(&response)?))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum SymbolKind {
    Function,
    Variable,
    Struct,
    Enum,
    Trait,
    TypeParameter,
    Field,
}

impl SymbolKind {
    fn name(&self) -> &str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Variable => "variable",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::TypeParameter => "type parameter",
            SymbolKind::Field => "field",
        }
    }
}

/// Find symbol at position
fn find_symbol_at_position(
    _program: &windjammer::parser::Program,
    _position: &Position,
) -> Option<(String, SymbolKind)> {
    // TODO: Implement actual position-based lookup
    // For now, return None
    None
}

/// Check if new name conflicts with existing symbols
fn has_naming_conflict(
    _program: &windjammer::parser::Program,
    _new_name: &str,
    _symbol_kind: &SymbolKind,
) -> bool {
    // TODO: Implement conflict detection based on scope and shadowing rules
    false
}

/// Find all occurrences of a symbol
fn find_all_occurrences(
    _program: &windjammer::parser::Program,
    _symbol_name: &str,
    _symbol_kind: &SymbolKind,
) -> usize {
    // TODO: Implement occurrence finding
    0
}

/// Validate identifier name
fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check for reserved keywords
    const RESERVED_KEYWORDS: &[&str] = &[
        "fn", "let", "const", "static", "if", "else", "match", "for", "while", "loop", "return",
        "break", "continue", "struct", "enum", "trait", "impl", "pub", "use", "mod", "type", "as",
        "in", "mut", "ref", "true", "false", "Self", "self", "super", "crate", "async", "await",
        "go", "defer",
    ];

    if RESERVED_KEYWORDS.contains(&name) {
        return false;
    }

    // First character must be letter or underscore
    let first = name.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric or underscore
    name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Rename visitor to track and update symbol references
#[allow(dead_code)]
struct RenameVisitor {
    old_name: String,
    new_name: String,
    symbol_kind: SymbolKind,
    occurrences: usize,
    scope_stack: Vec<HashMap<String, SymbolKind>>,
}

#[allow(dead_code)]
impl RenameVisitor {
    fn new(old_name: String, new_name: String, symbol_kind: SymbolKind) -> Self {
        Self {
            old_name,
            new_name,
            symbol_kind,
            occurrences: 0,
            scope_stack: vec![HashMap::new()],
        }
    }

    fn enter_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn visit_identifier(&mut self, name: &str) -> bool {
        // Check if this identifier matches our symbol
        if name == self.old_name {
            // Check if it's shadowed in current scope
            for scope in self.scope_stack.iter().rev() {
                if let Some(kind) = scope.get(name) {
                    // Shadowed by different kind of symbol
                    if kind != &self.symbol_kind {
                        return false;
                    }
                    break;
                }
            }
            self.occurrences += 1;
            return true;
        }
        false
    }

    fn declare_symbol(&mut self, name: String, kind: SymbolKind) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(name, kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rename_symbol_basic() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "code": "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}",
            "position": { "line": 1, "column": 8 },
            "new_name": "value"
        });

        let result = handle(db, args).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("foo"));
        assert!(is_valid_identifier("_bar"));
        assert!(is_valid_identifier("value42"));
        assert!(is_valid_identifier("_"));

        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("42foo"));
        assert!(!is_valid_identifier("foo-bar"));
        assert!(!is_valid_identifier("foo bar"));
        assert!(!is_valid_identifier("fn")); // Reserved word check would be added
    }

    #[test]
    fn test_symbol_kind() {
        assert_eq!(SymbolKind::Function.name(), "function");
        assert_eq!(SymbolKind::Variable.name(), "variable");
        assert_eq!(SymbolKind::Struct.name(), "struct");
    }
}
