// UI AST Extensions
//
// Additional AST types specific to UI code generation

use crate::parser::ast::{Expression, Statement};

/// UI widget call (e.g., Button, Text, VStack)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UIWidget {
    pub name: String,                     // Widget name: "Button", "Text", "VStack"
    pub props: Vec<(String, Expression)>, // Named props: [("text", "Hello"), ("on_click", closure)]
    pub children: Vec<UINode>,            // Child widgets
}

/// UI node in the widget tree
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UINode {
    Widget(UIWidget),       // A widget like Button, Text
    Expression(Expression), // An expression that evaluates to UI
    Conditional {
        // if/else in UI
        condition: Expression,
        then_ui: Box<UINode>,
        else_ui: Option<Box<UINode>>,
    },
    Loop {
        // for loop in UI
        pattern: String,
        iterator: Expression,
        body: Box<UINode>,
    },
}

/// Signal (reactive state)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignalDecl {
    pub name: String,
    pub initial_value: Expression,
}

/// Memo (computed value)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoDecl {
    pub name: String,
    pub computation: Expression, // Usually a closure
}

/// Effect (side effect)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectDecl {
    pub callback: Expression, // Usually a closure
}

/// Extract UI widgets from component body
pub fn extract_ui_widgets(statements: &[Statement]) -> Vec<UINode> {
    let mut nodes = Vec::new();

    for stmt in statements {
        if let Some(node) = statement_to_ui_node(stmt) {
            nodes.push(node);
        }
    }

    nodes
}

/// Convert a statement to a UI node if it's a widget call
fn statement_to_ui_node(stmt: &Statement) -> Option<UINode> {
    match stmt {
        Statement::Expression { expr, location: _ } => expression_to_ui_node(expr),
        _ => None,
    }
}

/// Convert an expression to a UI node if it's a widget call
fn expression_to_ui_node(expr: &Expression) -> Option<UINode> {
    match expr {
        // Macro invocation like VStack { ... }
        Expression::MacroInvocation { name, args, .. } if is_ui_widget(name) => {
            // For macro invocations with {} delimiter, args are expressions to render
            // We can't extract full statements, so we convert args to UI nodes
            let children: Vec<UINode> = args.iter().filter_map(expression_to_ui_node).collect();

            Some(UINode::Widget(UIWidget {
                name: name.clone(),
                props: vec![],
                children,
            }))
        }

        // Function call like Button("Click me", on_click: || ...)
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            // Extract function name from expression
            if let Expression::Identifier { name, .. } = function.as_ref() {
                if is_ui_widget(name) {
                    return Some(UINode::Widget(UIWidget {
                        name: name.clone(),
                        props: extract_props_from_args(arguments),
                        children: vec![],
                    }));
                }
            }
            None
        }

        _ => None,
    }
}

/// Check if a name is a known UI widget
fn is_ui_widget(name: &str) -> bool {
    matches!(
        name,
        "Button"
            | "Text"
            | "TextInput"
            | "Label"
            | "Checkbox"
            | "RadioButton"
            | "ComboBox"
            | "Slider"
            | "RangeSlider"
            | "ColorPicker"
            | "TreeView"
            | "Image"
            | "Canvas"
            | "VStack"
            | "HStack"
            | "Grid"
            | "ScrollArea"
            | "SplitPanel"
            | "Spacer"
            | "Separator"
            | "Modal"
            | "ContextMenu"
            | "Tooltip"
            | "IconButton"
            | "Icon"
    )
}

/// Extract props from function call arguments
/// Arguments are Vec<(Option<String>, Expression)> where first element is optional label
fn extract_props_from_args(args: &[(Option<String>, Expression)]) -> Vec<(String, Expression)> {
    let mut props = Vec::new();

    for (i, (label, expr)) in args.iter().enumerate() {
        match label {
            // Named argument: name: value
            Some(name) => {
                props.push((name.clone(), expr.clone()));
            }
            // Positional argument: use index
            None => {
                props.push((format!("arg{}", i), expr.clone()));
            }
        }
    }

    props
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Expression, Statement};

    #[test]
    fn test_is_ui_widget() {
        assert!(is_ui_widget("Button"));
        assert!(is_ui_widget("VStack"));
        assert!(is_ui_widget("ColorPicker"));
        assert!(is_ui_widget("Text"));
        assert!(is_ui_widget("Slider"));
        assert!(is_ui_widget("TreeView"));
        assert!(!is_ui_widget("println"));
        assert!(!is_ui_widget("some_function"));
        assert!(!is_ui_widget("calculate"));
    }

    #[test]
    fn test_extract_props_from_args() {
        // Test named argument
        let named_arg = (
            Some("text".to_string()),
            Expression::Literal {
                value: crate::parser::ast::Literal::String("Hello".to_string()),
                location: None,
            },
        );

        let props = extract_props_from_args(&[named_arg]);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].0, "text");

        // Test positional arguments
        let pos_arg1 = (
            None,
            Expression::Literal {
                value: crate::parser::ast::Literal::String("Click me".to_string()),
                location: None,
            },
        );
        let pos_arg2 = (
            None,
            Expression::Literal {
                value: crate::parser::ast::Literal::Int(42),
                location: None,
            },
        );

        let props = extract_props_from_args(&[pos_arg1, pos_arg2]);
        assert_eq!(props.len(), 2);
        assert_eq!(props[0].0, "arg0");
        assert_eq!(props[1].0, "arg1");
    }

    #[test]
    fn test_expression_to_ui_node_macro_invocation() {
        use crate::parser::ast::MacroDelimiter;

        let macro_expr = Expression::MacroInvocation {
            name: "VStack".to_string(),
            args: vec![],
            delimiter: MacroDelimiter::Braces,
            location: None,
        };

        let node = expression_to_ui_node(&macro_expr);
        assert!(node.is_some());

        if let Some(UINode::Widget(widget)) = node {
            assert_eq!(widget.name, "VStack");
            assert_eq!(widget.children.len(), 0);
        } else {
            panic!("Expected Widget node");
        }
    }

    #[test]
    fn test_expression_to_ui_node_function_call() {
        let call_expr = Expression::Call {
            function: Box::new(Expression::Identifier {
                name: "Button".to_string(),
                location: None,
            }),
            arguments: vec![(
                None,
                Expression::Literal {
                    value: crate::parser::ast::Literal::String("Click".to_string()),
                    location: None,
                },
            )],
            location: None,
        };

        let node = expression_to_ui_node(&call_expr);
        assert!(node.is_some());

        if let Some(UINode::Widget(widget)) = node {
            assert_eq!(widget.name, "Button");
        } else {
            panic!("Expected Widget node");
        }
    }

    #[test]
    fn test_expression_to_ui_node_non_widget() {
        // Non-widget expressions should return None
        let identifier = Expression::Identifier {
            name: "some_variable".to_string(),
            location: None,
        };

        let node = expression_to_ui_node(&identifier);
        assert!(node.is_none());

        let literal = Expression::Literal {
            value: crate::parser::ast::Literal::Int(42),
            location: None,
        };

        let node = expression_to_ui_node(&literal);
        assert!(node.is_none());
    }

    #[test]
    fn test_extract_ui_widgets_empty() {
        let statements: Vec<Statement> = vec![];
        let nodes = extract_ui_widgets(&statements);
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_extract_ui_widgets_with_widgets() {
        use crate::parser::ast::MacroDelimiter;

        let statements = vec![
            Statement::Expression {
                expr: Expression::MacroInvocation {
                    name: "VStack".to_string(),
                    args: vec![],
                    delimiter: MacroDelimiter::Braces,
                    location: None,
                },
                location: None,
            },
            Statement::Expression {
                expr: Expression::Call {
                    function: Box::new(Expression::Identifier {
                        name: "Button".to_string(),
                        location: None,
                    }),
                    arguments: vec![],
                    location: None,
                },
                location: None,
            },
        ];

        let nodes = extract_ui_widgets(&statements);
        assert_eq!(nodes.len(), 2);
    }
}
