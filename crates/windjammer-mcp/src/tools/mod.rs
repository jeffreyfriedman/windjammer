//! MCP tools for Windjammer code understanding, generation, and refactoring

pub mod analyze_ssr_routing;
pub mod analyze_types;
pub mod catalog;
pub mod explain_error;
pub mod generate_code;
pub mod generate_component;
pub mod generate_game_entity;
pub mod get_definition;
pub mod get_language_info;
pub mod parse_code;
pub mod registry;
pub mod search_workspace;

pub mod refactor_extract_function;
pub mod refactor_inline_variable;
pub mod refactor_rename_symbol;

pub use registry::{error_response, text_response, ToolHandler};

use crate::error::{McpError, McpResult};
use crate::protocol::{Tool, ToolCallResult, ToolContent};
use catalog::{all_tool_specs, input_schema_for};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

pub struct ToolRegistry {
    db: Arc<Mutex<WindjammerDatabase>>,
    handlers: HashMap<String, ToolHandler>,
    tools: Vec<Tool>,
}

impl ToolRegistry {
    pub fn new(db: Arc<Mutex<WindjammerDatabase>>) -> Self {
        let mut registry = Self {
            db: db.clone(),
            handlers: HashMap::new(),
            tools: Vec::new(),
        };
        registry.register_all_tools();
        registry
    }

    fn register_all_tools(&mut self) {
        for spec in all_tool_specs() {
            let schema = input_schema_for(spec.name)
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
            let handler = self.handler_for(spec.name);
            self.tools.push(Tool {
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                input_schema: schema,
            });
            self.handlers.insert(spec.name.to_string(), handler);
        }
    }

    fn handler_for(&self, name: &str) -> ToolHandler {
        let unknown_name = name.to_string();
        let db = self.db.clone();
        match name {
            "parse_code" => Box::new(move |d, args| Box::pin(parse_code::handle(d, args))),
            "analyze_types" => Box::new(move |d, args| Box::pin(analyze_types::handle(d, args))),
            "get_definition" => Box::new(move |d, args| Box::pin(get_definition::handle(d, args))),
            "generate_code" => Box::new(move |d, args| Box::pin(generate_code::handle(d, args))),
            "explain_error" => Box::new(move |d, args| Box::pin(explain_error::handle(d, args))),
            "search_workspace" => {
                Box::new(move |d, args| Box::pin(search_workspace::handle(d, args)))
            }
            "extract_function" => {
                Box::new(move |d, args| Box::pin(refactor_extract_function::handle(d, args)))
            }
            "inline_variable" => {
                Box::new(move |d, args| Box::pin(refactor_inline_variable::handle(d, args)))
            }
            "rename_symbol" => {
                Box::new(move |d, args| Box::pin(refactor_rename_symbol::handle(d, args)))
            }
            "generate_component" => Box::new(move |_d, args| {
                Box::pin(async move {
                    let result = generate_component::execute(args)
                        .map_err(|e: String| McpError::InternalError { message: e })?;
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
            "generate_game_entity" => Box::new(move |_d, args| {
                Box::pin(async move {
                    let result = generate_game_entity::execute(args)
                        .map_err(|e: String| McpError::InternalError { message: e })?;
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
            "analyze_ssr_routing" => Box::new(move |_d, args| {
                Box::pin(async move {
                    let result = analyze_ssr_routing::execute(args)
                        .map_err(|e: String| McpError::InternalError { message: e })?;
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
            "get_language_info" => {
                Box::new(move |d, args| Box::pin(get_language_info::handle(d, args)))
            }
            _ => Box::new(move |_d, _args| {
                let tool_name = unknown_name.clone();
                Box::pin(async move { Err(McpError::ToolNotFound { tool_name }) })
            }),
        }
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_matches_catalog() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let registry = ToolRegistry::new(db);
        assert_eq!(registry.list_tools().len(), all_tool_specs().len());
    }

    #[test]
    fn test_manifest_count_matches_catalog() {
        let manifest = catalog::manifest_json();
        assert_eq!(
            manifest["count"].as_u64(),
            Some(all_tool_specs().len() as u64)
        );
    }
}
