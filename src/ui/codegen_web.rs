// Web Code Generator (WASM backend)
//
// Generates Rust code compatible with WASM for web targets using windjammer-ui

use crate::parser::ast::FunctionDecl;
use anyhow::Result;

// Import the desktop codegen since windjammer-ui handles cross-platform
use super::codegen_desktop;

/// Generate WASM-compatible Rust code for a UI component
/// Note: windjammer-ui is cross-platform, so we use the same codegen as desktop
pub fn generate_component(func: &FunctionDecl) -> Result<String> {
    // windjammer-ui supports both desktop and web with the same API
    // Just generate the same Component code, and it will work on WASM
    let mut code = codegen_desktop::generate_component(func)?;

    // Add WASM-specific attributes if needed
    code.insert_str(0, "#[cfg(target_arch = \"wasm32\")]\n");

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Decorator, Expression, FunctionDecl, Literal, Statement, Type};

    fn create_test_component(name: &str, body: Vec<Statement>) -> FunctionDecl {
        FunctionDecl {
            is_pub: false,
            is_extern: false,
            name: name.to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![],
            return_type: Some(Type::Custom("UI".to_string())),
            body,
            parent_type: None,
        }
    }

    #[test]
    fn test_generate_state_struct_wasm() {
        let func = create_test_component("Counter", vec![]);
        let code = generate_component(&func);

        assert!(code.is_ok());
        let code_str = code.unwrap();

        // Should have WASM marker
        assert!(code_str.contains("#[cfg(target_arch = \"wasm32\")]"));

        // Should have windjammer-ui imports (from desktop codegen)
        assert!(code_str.contains("use windjammer_ui::"));
    }

    #[test]
    fn test_generate_state_struct_with_signal_wasm() {
        let func = create_test_component(
            "Counter",
            vec![Statement::Let {
                pattern: crate::parser::ast::Pattern::Identifier("count".to_string()),
                mutable: false,
                type_: None,
                value: Expression::Call {
                    function: Box::new(Expression::Identifier {
                        name: "signal".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        Expression::Literal {
                            value: Literal::Int(0),
                            location: None,
                        },
                    )],
                    location: None,
                },
                location: None,
            }],
        );

        let code = generate_component(&func);
        assert!(code.is_ok());

        let code_str = code.unwrap();
        assert!(code_str.contains("count: Signal<i32>"));
    }

    #[test]
    fn test_generate_render_function_wasm() {
        let func = create_test_component("Counter", vec![]);
        let code = generate_component(&func);

        assert!(code.is_ok());
        let code_str = code.unwrap();

        // Should have Component impl
        assert!(code_str.contains("impl Component for Counter"));
        assert!(code_str.contains("fn render(&self) -> VNode"));
    }

    #[test]
    fn test_wasm_imports() {
        let func = create_test_component("Counter", vec![]);
        let code = generate_component(&func);

        assert!(code.is_ok());
        let code_str = code.unwrap();

        // Should have windjammer-ui imports
        assert!(code_str.contains("use windjammer_ui::Signal"));
        assert!(code_str.contains("use windjammer_ui::component::Component"));
    }

    #[test]
    fn test_generate_component_wasm_complete() {
        let func = create_test_component("Counter", vec![]);
        let result = generate_component(&func);

        assert!(result.is_ok());
        let code = result.unwrap();

        // Should contain both wasm marker and Component impl
        assert!(code.contains("#[cfg(target_arch = \"wasm32\")]"));
        assert!(code.contains("impl Component for Counter"));
    }
}
