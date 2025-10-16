//! MCP tools for Windjammer code understanding, generation, and refactoring

pub mod analyze_types;
pub mod explain_error;
pub mod generate_code;
pub mod get_definition;
pub mod parse_code;
pub mod search_workspace;

use crate::error::{McpError, McpResult};
use crate::protocol::{Tool, ToolCallResult, ToolContent};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

/// Type alias for tool handler function
type ToolHandler = Box<
    dyn Fn(
            Arc<Mutex<WindjammerDatabase>>,
            Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = McpResult<ToolCallResult>> + Send>>
        + Send
        + Sync,
>;

/// Registry of all available MCP tools
pub struct ToolRegistry {
    db: Arc<Mutex<WindjammerDatabase>>,
    handlers: HashMap<String, ToolHandler>,
    tools: Vec<Tool>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new(db: Arc<Mutex<WindjammerDatabase>>) -> Self {
        let mut registry = Self {
            db: db.clone(),
            handlers: HashMap::new(),
            tools: Vec::new(),
        };

        registry.register_all_tools();
        registry
    }

    /// Register all available tools
    fn register_all_tools(&mut self) {
        use serde_json::json;

        // Code understanding tools
        self.register_tool(
            "parse_code",
            "Parse Windjammer code and return AST structure",
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Windjammer source code to parse"
                    },
                    "include_diagnostics": {
                        "type": "boolean",
                        "description": "Include parse errors and warnings",
                        "default": true
                    }
                },
                "required": ["code"]
            }),
            Box::new(|db, args| Box::pin(parse_code::handle(db, args))),
        );

        self.register_tool(
            "analyze_types",
            "Perform type inference and analysis on Windjammer code",
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Windjammer source code to analyze"
                    },
                    "cursor_position": {
                        "type": "object",
                        "properties": {
                            "line": {"type": "integer"},
                            "column": {"type": "integer"}
                        },
                        "description": "Optional cursor position for type-at-point query"
                    }
                },
                "required": ["code"]
            }),
            Box::new(|db, args| Box::pin(analyze_types::handle(db, args))),
        );

        self.register_tool(
            "get_definition",
            "Find the definition of a symbol at a given position",
            json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "Symbol name to find"
                    },
                    "position": {
                        "type": "object",
                        "properties": {
                            "line": {"type": "integer"},
                            "column": {"type": "integer"}
                        }
                    }
                },
                "required": ["symbol"]
            }),
            Box::new(|db, args| Box::pin(get_definition::handle(db, args))),
        );

        // Code generation tools
        self.register_tool(
            "generate_code",
            "Generate Windjammer code from natural language description",
            json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Natural language description of desired code"
                    },
                    "context": {
                        "type": "object",
                        "description": "Optional context (existing functions, imports, etc.)"
                    }
                },
                "required": ["description"]
            }),
            Box::new(|db, args| Box::pin(generate_code::handle(db, args))),
        );

        // Error handling tools
        self.register_tool(
            "explain_error",
            "Explain a Windjammer compiler error in plain English",
            json!({
                "type": "object",
                "properties": {
                    "error": {
                        "type": "string",
                        "description": "Error message from compiler"
                    },
                    "code_context": {
                        "type": "string",
                        "description": "Surrounding code context"
                    }
                },
                "required": ["error"]
            }),
            Box::new(|db, args| Box::pin(explain_error::handle(db, args))),
        );

        // Workspace tools
        self.register_tool(
            "search_workspace",
            "Search for code patterns across the workspace",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query or code pattern"
                    },
                    "file_pattern": {
                        "type": "string",
                        "description": "File glob pattern (e.g., 'src/**/*.wj')",
                        "default": "**/*.wj"
                    }
                },
                "required": ["query"]
            }),
            Box::new(|db, args| Box::pin(search_workspace::handle(db, args))),
        );
    }

    /// Register a single tool
    fn register_tool(
        &mut self,
        name: &str,
        description: &str,
        input_schema: Value,
        handler: ToolHandler,
    ) {
        self.tools.push(Tool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        });

        self.handlers.insert(name.to_string(), handler);
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    /// Call a tool by name
    pub async fn call_tool(&self, name: &str, arguments: Value) -> McpResult<ToolCallResult> {
        let handler = self
            .handlers
            .get(name)
            .ok_or_else(|| McpError::ToolNotFound {
                tool_name: name.to_string(),
            })?;

        handler(self.db.clone(), arguments).await
    }
}

/// Helper function to create a text response
pub fn text_response(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::Text { text: text.into() }],
        is_error: false,
    }
}

/// Helper function to create an error response
pub fn error_response(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::Text { text: text.into() }],
        is_error: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let registry = ToolRegistry::new(db);

        let tools = registry.list_tools();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "parse_code"));
        assert!(tools.iter().any(|t| t.name == "generate_code"));
    }
}
