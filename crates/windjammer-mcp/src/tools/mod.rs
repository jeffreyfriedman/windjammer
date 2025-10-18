//! MCP tools for Windjammer code understanding, generation, and refactoring

pub mod analyze_types;
pub mod explain_error;
pub mod generate_code;
pub mod get_definition;
pub mod parse_code;
pub mod search_workspace;

// Refactoring tools
pub mod refactor_extract_function;
pub mod refactor_inline_variable;
pub mod refactor_rename_symbol;

// UI Framework tools
pub mod analyze_ssr_routing;
pub mod generate_component;
pub mod generate_game_entity;

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

        // Refactoring tools
        self.register_tool(
            "extract_function",
            "Extract selected code into a new function",
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Source code to refactor"
                    },
                    "range": {
                        "type": "object",
                        "properties": {
                            "start": {
                                "type": "object",
                                "properties": {
                                    "line": {"type": "integer"},
                                    "column": {"type": "integer"}
                                }
                            },
                            "end": {
                                "type": "object",
                                "properties": {
                                    "line": {"type": "integer"},
                                    "column": {"type": "integer"}
                                }
                            }
                        },
                        "description": "Selection range to extract"
                    },
                    "function_name": {
                        "type": "string",
                        "description": "Name for the new function"
                    },
                    "make_public": {
                        "type": "boolean",
                        "description": "Make function public",
                        "default": false
                    }
                },
                "required": ["code", "range", "function_name"]
            }),
            Box::new(|db, args| Box::pin(refactor_extract_function::handle(db, args))),
        );

        self.register_tool(
            "inline_variable",
            "Inline a variable by replacing all uses with its value",
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Source code to refactor"
                    },
                    "position": {
                        "type": "object",
                        "properties": {
                            "line": {"type": "integer"},
                            "column": {"type": "integer"}
                        },
                        "description": "Position of variable to inline"
                    },
                    "variable_name": {
                        "type": "string",
                        "description": "Optional variable name"
                    }
                },
                "required": ["code", "position"]
            }),
            Box::new(|db, args| Box::pin(refactor_inline_variable::handle(db, args))),
        );

        self.register_tool(
            "rename_symbol",
            "Rename a symbol with workspace-wide updates",
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Source code to refactor"
                    },
                    "position": {
                        "type": "object",
                        "properties": {
                            "line": {"type": "integer"},
                            "column": {"type": "integer"}
                        },
                        "description": "Position of symbol to rename"
                    },
                    "new_name": {
                        "type": "string",
                        "description": "New name for the symbol"
                    },
                    "old_name": {
                        "type": "string",
                        "description": "Optional current name"
                    }
                },
                "required": ["code", "position", "new_name"]
            }),
            Box::new(|db, args| Box::pin(refactor_rename_symbol::handle(db, args))),
        );

        // UI Framework tools
        self.register_tool(
            "generate_component",
            "Generate a Windjammer UI component with @component decorator",
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Component name (PascalCase)"
                    },
                    "props": {
                        "type": "array",
                        "description": "Component properties"
                    },
                    "render_template": {
                        "type": "string",
                        "enum": ["div", "button", "form", "list", "card", "custom"]
                    }
                },
                "required": ["name"]
            }),
            Box::new(|_db, args| {
                Box::pin(async move {
                    let result = generate_component::execute(args).map_err(|e: &str| {
                        McpError::InternalError {
                            message: e.to_string(),
                        }
                    })?;
                    Ok(ToolCallResult {
                        content: result
                            .into_iter()
                            .map(|v| ToolContent::Text {
                                text: v["text"].as_str().unwrap_or("").to_string(),
                            })
                            .collect(),
                        is_error: false,
                    })
                })
            }),
        );

        self.register_tool(
            "generate_game_entity",
            "Generate a game entity with @game decorator and ECS components",
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Entity name"
                    },
                    "entity_type": {
                        "type": "string",
                        "enum": ["player", "enemy", "projectile", "item", "npc", "custom"]
                    },
                    "include_physics": {
                        "type": "boolean"
                    },
                    "include_health": {
                        "type": "boolean"
                    }
                },
                "required": ["name", "entity_type"]
            }),
            Box::new(|_db, args| {
                Box::pin(async move {
                    let result = generate_game_entity::execute(args).map_err(|e: &str| {
                        McpError::InternalError {
                            message: e.to_string(),
                        }
                    })?;
                    Ok(ToolCallResult {
                        content: result
                            .into_iter()
                            .map(|v| ToolContent::Text {
                                text: v["text"].as_str().unwrap_or("").to_string(),
                            })
                            .collect(),
                        is_error: false,
                    })
                })
            }),
        );

        self.register_tool(
            "analyze_ssr_routing",
            "Analyze SSR and routing configurations",
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Code to analyze"
                    },
                    "analysis_type": {
                        "type": "string",
                        "enum": ["ssr", "routing", "both"]
                    },
                    "check_hydration": {
                        "type": "boolean"
                    },
                    "check_seo": {
                        "type": "boolean"
                    }
                },
                "required": ["code"]
            }),
            Box::new(|_db, args| {
                Box::pin(async move {
                    let result = analyze_ssr_routing::execute(args).map_err(|e: &str| {
                        McpError::InternalError {
                            message: e.to_string(),
                        }
                    })?;
                    Ok(ToolCallResult {
                        content: result
                            .into_iter()
                            .map(|v| ToolContent::Text {
                                text: v["text"].as_str().unwrap_or("").to_string(),
                            })
                            .collect(),
                        is_error: false,
                    })
                })
            }),
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
