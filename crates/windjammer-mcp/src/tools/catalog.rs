//! MCP tool catalog — metadata and JSON Schema generation (single source of truth).

use schemars::schema_for;
use serde_json::{json, Value};

use super::registry::ToolStability;
use super::{
    analyze_ssr_routing, analyze_types, explain_error, generate_code, generate_component,
    generate_game_entity, get_definition, get_language_info, parse_code, refactor_extract_function,
    refactor_inline_variable, refactor_rename_symbol, search_workspace,
};

#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub stability: ToolStability,
}

pub fn all_tool_specs() -> &'static [ToolSpec] {
    &[
        ToolSpec {
            name: "parse_code",
            description: "Parse Windjammer code and return AST structure",
            category: "analyze",
            stability: ToolStability::Stable,
        },
        ToolSpec {
            name: "analyze_types",
            description: "Perform type inference and analysis on Windjammer code",
            category: "analyze",
            stability: ToolStability::Stable,
        },
        ToolSpec {
            name: "get_definition",
            description: "Find the definition of a symbol at a given position",
            category: "analyze",
            stability: ToolStability::Beta,
        },
        ToolSpec {
            name: "generate_code",
            description: "Generate Windjammer code from natural language description",
            category: "generate",
            stability: ToolStability::Beta,
        },
        ToolSpec {
            name: "explain_error",
            description: "Explain a Windjammer compiler error in plain English",
            category: "knowledge",
            stability: ToolStability::Stable,
        },
        ToolSpec {
            name: "search_workspace",
            description: "Search for code patterns across the workspace",
            category: "analyze",
            stability: ToolStability::Beta,
        },
        ToolSpec {
            name: "extract_function",
            description: "Extract selected code into a new function",
            category: "refactor",
            stability: ToolStability::Stable,
        },
        ToolSpec {
            name: "inline_variable",
            description: "Inline a variable by replacing all uses with its value",
            category: "refactor",
            stability: ToolStability::Stable,
        },
        ToolSpec {
            name: "rename_symbol",
            description: "Rename a symbol with workspace-wide updates",
            category: "refactor",
            stability: ToolStability::Stable,
        },
        ToolSpec {
            name: "generate_component",
            description: "Generate a Windjammer UI component with @component decorator",
            category: "generate",
            stability: ToolStability::Beta,
        },
        ToolSpec {
            name: "generate_game_entity",
            description: "Generate a game entity with @game decorator and ECS components",
            category: "generate",
            stability: ToolStability::Beta,
        },
        ToolSpec {
            name: "analyze_ssr_routing",
            description: "Analyze SSR and routing configurations",
            category: "analyze",
            stability: ToolStability::Beta,
        },
        ToolSpec {
            name: "get_language_info",
            description: "Get Windjammer compiler version, MCP version, and agent index metadata",
            category: "knowledge",
            stability: ToolStability::Stable,
        },
    ]
}

pub fn input_schema_for(name: &str) -> Option<Value> {
    let schema = match name {
        "parse_code" => schema_for!(parse_code::ParseCodeRequest),
        "analyze_types" => schema_for!(analyze_types::AnalyzeTypesRequest),
        "get_definition" => schema_for!(get_definition::GetDefinitionRequest),
        "generate_code" => schema_for!(generate_code::GenerateCodeRequest),
        "explain_error" => schema_for!(explain_error::ExplainErrorRequest),
        "search_workspace" => schema_for!(search_workspace::SearchWorkspaceRequest),
        "extract_function" => schema_for!(refactor_extract_function::ExtractFunctionRequest),
        "inline_variable" => schema_for!(refactor_inline_variable::InlineVariableRequest),
        "rename_symbol" => schema_for!(refactor_rename_symbol::RenameSymbolRequest),
        "generate_component" => schema_for!(generate_component::GenerateComponentArgs),
        "generate_game_entity" => schema_for!(generate_game_entity::GenerateGameEntityArgs),
        "analyze_ssr_routing" => schema_for!(analyze_ssr_routing::AnalyzeSsrRoutingArgs),
        "get_language_info" => schema_for!(get_language_info::GetLanguageInfoRequest),
        _ => return None,
    };
    Some(serde_json::to_value(schema.schema).unwrap_or(json!({"type": "object"})))
}

pub fn manifest_json() -> Value {
    let tools: Vec<Value> = all_tool_specs()
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "description": s.description,
                "category": s.category,
                "stability": s.stability.as_str(),
            })
        })
        .collect();
    json!({ "tools": tools, "count": tools.len() })
}
