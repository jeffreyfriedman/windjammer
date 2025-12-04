// Integration tests for Windjammer UI compilation
//
// These tests validate the complete UI compilation pipeline from
// Windjammer UI code to generated Rust code (both desktop and web targets)

use windjammer::parser::ast::{
    Decorator, Expression, FunctionDecl, Literal, OwnershipHint, Parameter, Statement, Type,
};
use windjammer::ui::codegen_desktop;
use windjammer::ui::codegen_web;
use windjammer::ui::type_checker::check_component;
use windjammer::ui::{get_component_decorator, is_ui_component, validate_component_return_type};

// Helper to validate generated code contains expected patterns
fn assert_code_contains(code: &str, patterns: &[&str]) {
    for pattern in patterns {
        assert!(
            code.contains(pattern),
            "Generated code missing pattern: '{}'\n\nGenerated code:\n{}",
            pattern,
            code
        );
    }
}

fn create_simple_counter_component() -> FunctionDecl {
    FunctionDecl {
        name: "Counter".to_string(),
        is_pub: false,
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![Decorator {
            name: "component".to_string(),
            arguments: vec![],
        }],
        is_async: false,
        is_extern: false,
        parameters: vec![],
        return_type: Some(Type::Custom("UI".to_string())),
        body: vec![
            Statement::Let {
                pattern: windjammer::parser::ast::Pattern::Identifier("count".to_string()),
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
                else_block: None,
                location: None,
            },
            Statement::Expression {
                expr: Expression::MacroInvocation {
                    name: "VStack".to_string(),
                    args: vec![
                        Expression::Call {
                            function: Box::new(Expression::Identifier {
                                name: "Text".to_string(),
                                location: None,
                            }),
                            arguments: vec![(
                                None,
                                Expression::Literal {
                                    value: Literal::String("Count: {count}".to_string()),
                                    location: None,
                                },
                            )],
                            location: None,
                        },
                        Expression::Call {
                            function: Box::new(Expression::Identifier {
                                name: "Button".to_string(),
                                location: None,
                            }),
                            arguments: vec![(
                                None,
                                Expression::Literal {
                                    value: Literal::String("Increment".to_string()),
                                    location: None,
                                },
                            )],
                            location: None,
                        },
                    ],
                    delimiter: windjammer::parser::ast::MacroDelimiter::Braces,
                    location: None,
                },
                location: None,
            },
        ],
        parent_type: None,
    }
}

fn create_component_with_props() -> FunctionDecl {
    FunctionDecl {
        name: "Greeting".to_string(),
        is_pub: false,
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![Decorator {
            name: "component".to_string(),
            arguments: vec![],
        }],
        is_async: false,
        is_extern: false,
        parameters: vec![
            Parameter {
                name: "name".to_string(),
                pattern: None,
                type_: Type::String,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
            },
            Parameter {
                name: "age".to_string(),
                pattern: None,
                type_: Type::Int,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
            },
        ],
        return_type: Some(Type::Custom("UI".to_string())),
        body: vec![Statement::Expression {
            expr: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "Text".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Literal {
                        value: Literal::String(
                            "Hello, {name}! You are {age} years old.".to_string(),
                        ),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }],
        parent_type: None,
    }
}

#[test]
fn test_component_detection() {
    let component = create_simple_counter_component();

    assert!(is_ui_component(&component));
    assert!(get_component_decorator(&component).is_some());

    let decorator = get_component_decorator(&component).unwrap();
    assert_eq!(decorator.name, "component");
}

#[test]
fn test_non_component_function() {
    let regular_func = FunctionDecl {
        name: "regular_function".to_string(),
        is_pub: false,
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        is_extern: false,
        parameters: vec![],
        return_type: Some(Type::Int),
        body: vec![],
        parent_type: None,
    };

    assert!(!is_ui_component(&regular_func));
    assert!(get_component_decorator(&regular_func).is_none());
}

#[test]
fn test_component_return_type_validation() {
    let valid_component = create_simple_counter_component();
    assert!(validate_component_return_type(&valid_component).is_ok());

    let invalid_component = FunctionDecl {
        name: "BadComponent".to_string(),
        is_pub: false,
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![Decorator {
            name: "component".to_string(),
            arguments: vec![],
        }],
        is_async: false,
        is_extern: false,
        parameters: vec![],
        return_type: Some(Type::Int),
        body: vec![],
        parent_type: None,
    };

    assert!(validate_component_return_type(&invalid_component).is_err());
}

#[test]
fn test_type_checker_validates_component() {
    let component = create_simple_counter_component();
    assert!(check_component(&component).is_ok());
}

#[test]
fn test_type_checker_validates_props() {
    let component = create_component_with_props();
    assert!(check_component(&component).is_ok());
}

#[test]
fn test_type_checker_rejects_inferred_props() {
    let bad_component = FunctionDecl {
        name: "BadComponent".to_string(),
        is_pub: false,
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![Decorator {
            name: "component".to_string(),
            arguments: vec![],
        }],
        is_async: false,
        is_extern: false,
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

    assert!(check_component(&bad_component).is_err());
}

#[test]
fn test_desktop_codegen_generates_valid_rust() {
    let component = create_simple_counter_component();
    let result = codegen_desktop::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Verify generated code contains expected structures (updated for windjammer-ui)
    assert!(code.contains("pub struct Counter"));
    assert!(code.contains("impl Counter"));
    assert!(code.contains("pub fn new() -> Self"));
    assert!(code.contains("impl Component for Counter"));
    assert!(code.contains("fn render(&self) -> VNode"));
    assert!(code.contains("use windjammer_ui::"));
}

#[test]
fn test_desktop_codegen_handles_signals() {
    let component = create_simple_counter_component();
    let result = codegen_desktop::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Should generate signal field in component struct (updated for windjammer-ui)
    assert!(code.contains("count: Signal<i32>"));
}

#[test]
fn test_desktop_codegen_handles_widgets() {
    let component = create_simple_counter_component();
    let result = codegen_desktop::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Should generate windjammer-ui widget calls (updated)
    assert!(code.contains("Flex::new()") && code.contains("FlexDirection::Column"));
}

#[test]
fn test_web_codegen_generates_valid_wasm() {
    let component = create_simple_counter_component();
    let result = codegen_web::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Web codegen now reuses desktop (windjammer-ui is cross-platform)
    assert!(code.contains("#[cfg(target_arch = \"wasm32\")]"));
    assert!(code.contains("impl Component for Counter"));
    assert!(code.contains("use windjammer_ui::"));
}

#[test]
fn test_web_codegen_handles_signals() {
    let component = create_simple_counter_component();
    let result = codegen_web::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Should generate signal field (updated for windjammer-ui)
    assert!(code.contains("count: Signal<i32>"));
}

#[test]
fn test_web_codegen_has_mount_function() {
    let component = create_simple_counter_component();
    let result = codegen_web::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Web now uses Component trait (no separate mount function needed)
    assert!(code.contains("impl Component for Counter"));
    assert!(code.contains("fn render(&self) -> VNode"));
}

#[test]
fn test_component_with_props_desktop() {
    let component = create_component_with_props();
    let result = codegen_desktop::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Should generate struct and Component impl (updated for windjammer-ui)
    assert!(code.contains("Greeting"));
    assert!(code.contains("impl Component for Greeting"));
}

#[test]
fn test_component_with_props_web() {
    let component = create_component_with_props();
    let result = codegen_web::generate_component(&component);

    assert!(result.is_ok());
    let code = result.unwrap();

    // Should generate struct (updated for windjammer-ui)
    assert!(code.contains("Greeting"));
    assert!(code.contains("impl Component for Greeting"));
}

#[test]
fn test_full_pipeline_desktop() {
    // 1. Create component
    let component = create_simple_counter_component();

    // 2. Validate it's a component
    assert!(is_ui_component(&component));

    // 3. Type check
    assert!(check_component(&component).is_ok());

    // 4. Generate desktop code
    let desktop_code = codegen_desktop::generate_component(&component);
    assert!(desktop_code.is_ok());

    // 5. Verify code quality
    let code = desktop_code.unwrap();
    assert!(!code.is_empty());
    assert!(code.contains("windjammer_ui")); // Updated: uses windjammer-ui instead of egui
}

#[test]
fn test_full_pipeline_web() {
    // 1. Create component
    let component = create_simple_counter_component();

    // 2. Validate it's a component
    assert!(is_ui_component(&component));

    // 3. Type check
    assert!(check_component(&component).is_ok());

    // 4. Generate web code
    let web_code = codegen_web::generate_component(&component);
    assert!(web_code.is_ok());

    // 5. Verify code quality (updated for windjammer-ui)
    let code = web_code.unwrap();
    assert!(!code.is_empty());
    assert!(code.contains("#[cfg(target_arch = \"wasm32\")]"));
    assert!(code.contains("impl Component for Counter"));
}

/// Test that codegen generates correct windjammer-ui imports and API calls
#[test]
fn test_windjammer_ui_integration() {
    let component = create_simple_counter_component();

    // Generate desktop code
    let result = codegen_desktop::generate_component(&component);
    assert!(result.is_ok(), "Code generation failed: {:?}", result.err());

    let code = result.unwrap();

    // Verify windjammer-ui imports
    assert_code_contains(
        &code,
        &[
            "use windjammer_ui::Signal",
            "use windjammer_ui::component::Component",
            "use windjammer_ui::vdom::VNode",
            "use windjammer_ui::components::button::Button",
            "use windjammer_ui::components::text::Text",
            "use windjammer_ui::components::flex::{Flex, FlexDirection}",
        ],
    );

    // Verify component struct
    assert_code_contains(&code, &["pub struct Counter", "count: Signal<i32>"]);

    // Verify constructor
    assert_code_contains(
        &code,
        &["impl Counter", "pub fn new() -> Self", "Signal::new(0)"],
    );

    // Verify Component trait impl
    assert_code_contains(
        &code,
        &["impl Component for Counter", "fn render(&self) -> VNode"],
    );

    // Verify widget API calls
    assert_code_contains(
        &code,
        &[
            "Text::new",
            ".render()",
            "Button::new",
            "Flex::new()",
            "FlexDirection::Column",
        ],
    );
}

/// Test that web codegen properly reuses desktop codegen
#[test]
fn test_web_codegen_reuses_desktop() {
    let component = create_simple_counter_component();

    let web_code = codegen_web::generate_component(&component);
    assert!(web_code.is_ok());

    let code = web_code.unwrap();

    // Should contain WASM marker (currently adds it at the top)
    // Note: web codegen reuses desktop, so check for Component impl
    assert!(code.contains("impl Component for Counter"));
    assert!(code.contains("use windjammer_ui::Signal"));
}
