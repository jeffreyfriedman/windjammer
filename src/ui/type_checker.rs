// UI Type Checker
//
// Type checking for UI components

use crate::parser::ast::{FunctionDecl, Type};
use anyhow::{anyhow, Result};

/// Type check a UI component function
pub fn check_component(func: &FunctionDecl) -> Result<()> {
    // 1. Must return UI type
    validate_return_type(func)?;

    // 2. Props must be properly typed
    validate_props(func)?;

    // 3. Signal/memo/effect usage must be type-safe
    // TODO: Implement comprehensive type checking for signals

    Ok(())
}

/// Validate that component returns UI type
fn validate_return_type(func: &FunctionDecl) -> Result<()> {
    match &func.return_type {
        Some(Type::Custom(name)) if name == "UI" => Ok(()),
        Some(other) => Err(anyhow!(
            "Component '{}' must return UI type, found: {:?}",
            func.name,
            other
        )),
        None => Err(anyhow!(
            "Component '{}' must have explicit UI return type",
            func.name
        )),
    }
}

/// Validate component props (parameters)
fn validate_props(func: &FunctionDecl) -> Result<()> {
    for param in &func.parameters {
        // All props must have explicit types
        if matches!(param.type_, Type::Infer) {
            return Err(anyhow!(
                "Component '{}' parameter '{}' must have explicit type",
                func.name,
                param.name
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Decorator, OwnershipHint, Parameter};

    #[test]
    fn test_validate_return_type() {
        let valid = FunctionDecl {
            is_pub: false,
            is_extern: false,
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

        assert!(validate_return_type(&valid).is_ok());
    }

    #[test]
    fn test_validate_props() {
        let valid = FunctionDecl {
            is_pub: false,
            is_extern: false,
            name: "Greeting".to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![Parameter {
                name: "name".to_string(),
                pattern: None,
                type_: Type::String,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
            }],
            return_type: Some(Type::Custom("UI".to_string())),
            body: vec![],
            parent_type: None,
        };

        assert!(validate_props(&valid).is_ok());

        let invalid = FunctionDecl {
            is_pub: false,
            is_extern: false,
            name: "BadComponent".to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![Parameter {
                name: "value".to_string(),
                pattern: None,
                type_: Type::Infer,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
            }],
            return_type: Some(Type::Custom("UI".to_string())),
            body: vec![],
            parent_type: None,
        };

        assert!(validate_props(&invalid).is_err());
    }
}
