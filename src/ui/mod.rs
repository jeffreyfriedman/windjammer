// UI Code Generation Module
//
// This module handles compilation of Windjammer UI code (@component decorators)
// to both desktop (egui) and web (WASM) targets.

pub mod ast_extensions;
pub mod codegen_desktop;
pub mod codegen_web;
pub mod parser;
pub mod type_checker;

use crate::parser::ast::{Decorator, FunctionDecl};
use anyhow::{anyhow, Result};

/// Compilation target for UI code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UITarget {
    Desktop, // Compiles to Rust + egui
    Web,     // Compiles to Rust + WASM
}

/// Check if a function is a UI component (has @component decorator)
pub fn is_ui_component(func: &FunctionDecl) -> bool {
    func.decorators
        .iter()
        .any(|d| d.name == "component" || d.name == "ui_component")
}

/// Get the @component decorator from a function if it exists
pub fn get_component_decorator(func: &FunctionDecl) -> Option<&Decorator> {
    func.decorators
        .iter()
        .find(|d| d.name == "component" || d.name == "ui_component")
}

/// Validate that a function with @component decorator returns UI type
pub fn validate_component_return_type(func: &FunctionDecl) -> Result<()> {
    match &func.return_type {
        Some(crate::parser::ast::Type::Custom(name)) if name == "UI" => Ok(()),
        Some(_) => Err(anyhow!(
            "Component function '{}' must return UI type, found: {:?}",
            func.name,
            func.return_type
        )),
        None => Err(anyhow!(
            "Component function '{}' must have explicit UI return type",
            func.name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Decorator, FunctionDecl, Type};

    #[test]
    fn test_is_ui_component() {
        let component = FunctionDecl {
            name: "MyComponent".to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![],
            return_type: Some(Type::Custom("UI".to_string())),
            body: vec![],
            parent_type: None,
        };

        assert!(is_ui_component(&component));
    }

    #[test]
    fn test_validate_component_return_type() {
        let valid_component = FunctionDecl {
            name: "MyComponent".to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![],
            return_type: Some(Type::Custom("UI".to_string())),
            body: vec![],
            parent_type: None,
        };

        assert!(validate_component_return_type(&valid_component).is_ok());

        let invalid_component = FunctionDecl {
            name: "BadComponent".to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![],
            return_type: Some(Type::String),
            body: vec![],
            parent_type: None,
        };

        assert!(validate_component_return_type(&invalid_component).is_err());
    }
}
