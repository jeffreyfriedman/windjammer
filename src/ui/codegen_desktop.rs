// Desktop Code Generator (windjammer-ui backend)
//
// Generates Rust code using windjammer-ui for desktop targets

use crate::parser::ast::{Expression, FunctionDecl, Literal};
use crate::ui::ast_extensions::{extract_ui_widgets, UINode, UIWidget};
use crate::ui::parser::{extract_memos, extract_signals};
use anyhow::Result;

/// Generate windjammer-ui Rust code for a UI component
pub fn generate_component(func: &FunctionDecl) -> Result<String> {
    let mut code = String::new();

    // Generate imports
    code.push_str(&generate_imports());
    code.push_str("\n\n");

    // Generate component struct
    code.push_str(&generate_component_struct(func));
    code.push_str("\n\n");

    // Generate constructor
    code.push_str(&generate_constructor(func));
    code.push_str("\n\n");

    // Generate Component trait implementation
    code.push_str(&generate_component_impl(func));

    Ok(code)
}

/// Generate imports for windjammer-ui
fn generate_imports() -> String {
    r#"use windjammer_ui::Signal;
use windjammer_ui::component::Component;
use windjammer_ui::vdom::VNode;
use windjammer_ui::components::button::Button;
use windjammer_ui::components::text::Text;
use windjammer_ui::components::flex::{Flex, FlexDirection};"#
        .to_string()
}

/// Generate the component struct
fn generate_component_struct(func: &FunctionDecl) -> String {
    let signals = extract_signals(func);
    let memos = extract_memos(func);

    let mut code = format!("pub struct {} {{\n", func.name);

    // Add signal fields
    for signal in &signals {
        // Infer type from initial value
        let type_str = infer_type_from_expr(&signal.initial_value);
        code.push_str(&format!("    {}: Signal<{}>,\n", signal.name, type_str));
    }

    // Add memo fields
    for memo in &memos {
        code.push_str(&format!(
            "    {}: Signal<String>, // TODO: infer memo type\n",
            memo.name
        ));
    }

    code.push_str("}\n");

    code
}

/// Generate constructor for the component
fn generate_constructor(func: &FunctionDecl) -> String {
    let signals = extract_signals(func);
    let memos = extract_memos(func);

    let mut code = format!("impl {} {{\n", func.name);
    code.push_str("    pub fn new() -> Self {\n");
    code.push_str("        Self {\n");

    for signal in &signals {
        let init_value = generate_expression(&signal.initial_value);
        code.push_str(&format!(
            "            {}: Signal::new({}),\n",
            signal.name, init_value
        ));
    }

    for memo in &memos {
        code.push_str(&format!(
            "            {}: Signal::new(String::new()), // TODO: memo computation\n",
            memo.name
        ));
    }

    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// Infer Rust type from an expression
fn infer_type_from_expr(expr: &Expression) -> String {
    match expr {
        Expression::Literal { value, .. } => match value {
            Literal::Int(_) => "i32".to_string(),
            Literal::Float(_) => "f64".to_string(),
            Literal::String(_) => "String".to_string(),
            Literal::Bool(_) => "bool".to_string(),
            Literal::Char(_) => "char".to_string(),
        },
        _ => "i32".to_string(), // Default
    }
}

/// Generate Rust code for an expression
fn generate_expression(expr: &Expression) -> String {
    match expr {
        Expression::Literal { value, .. } => match value {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Bool(b) => b.to_string(),
            Literal::Char(c) => format!("'{}'", c),
        },
        Expression::Identifier { name, .. } => name.clone(),
        _ => "/* complex expression */".to_string(),
    }
}

/// Generate Component trait implementation
fn generate_component_impl(func: &FunctionDecl) -> String {
    let mut code = format!("impl Component for {} {{\n", func.name);
    code.push_str("    fn render(&self) -> VNode {\n");

    // Generate UI widgets
    let ui_nodes = extract_ui_widgets(&func.body);

    if ui_nodes.len() == 1 {
        // Single widget
        code.push_str(&generate_ui_node(&ui_nodes[0], 2));
    } else {
        // Multiple widgets - wrap in VStack
        code.push_str("        Flex::new()\n");
        code.push_str("            .direction(FlexDirection::Column)\n");
        code.push_str("            .children(vec![\n");
        for node in &ui_nodes {
            code.push_str("                ");
            code.push_str(generate_ui_node(node, 4).trim_start());
        }
        code.push_str("            ])\n");
        code.push_str("            .render()\n");
    }

    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// Generate code for a UI node (returns VNode)
fn generate_ui_node(node: &UINode, indent_level: usize) -> String {
    match node {
        UINode::Widget(widget) => generate_widget(widget, indent_level),
        UINode::Expression(expr) => {
            // Convert expression to text
            format!(
                "Text::new(format!(\"{{:?}}\", {})).render(),\n",
                generate_expression(expr)
            )
        }
        UINode::Conditional {
            condition,
            then_ui,
            else_ui,
        } => {
            // Conditionals need to return VNode
            let mut code = String::from("if ");
            code.push_str(&generate_expression(condition));
            code.push_str(" { ");
            code.push_str(generate_ui_node(then_ui, indent_level).trim());
            code.push_str(" } else { ");
            if let Some(else_branch) = else_ui {
                code.push_str(generate_ui_node(else_branch, indent_level).trim());
            } else {
                code.push_str("VNode::Empty");
            }
            code.push_str(" },\n");
            code
        }
        UINode::Loop {
            pattern,
            iterator: _,
            body: _,
        } => {
            // Loops collect VNodes
            format!(
                "/* loop over {} */ vec![].into_iter().collect::<Vec<VNode>>(),\n",
                pattern
            )
        }
    }
}

/// Generate code for a UI widget
fn generate_widget(widget: &UIWidget, indent_level: usize) -> String {
    let indent = "    ".repeat(indent_level);

    match widget.name.as_str() {
        "Text" => generate_text_widget(widget, indent_level),
        "Button" => generate_button_widget(widget, indent_level),
        "TextInput" => generate_text_input_widget(widget, indent_level),
        "Checkbox" => generate_checkbox_widget(widget, indent_level),
        "Slider" => generate_slider_widget(widget, indent_level),
        "ColorPicker" => generate_color_picker_widget(widget, indent_level),
        "VStack" => generate_vstack_widget(widget, indent_level),
        "HStack" => generate_hstack_widget(widget, indent_level),
        "ScrollArea" => generate_scroll_area_widget(widget, indent_level),
        _ => format!("{}// TODO: Widget {}\n", indent, widget.name),
    }
}

fn generate_text_widget(widget: &UIWidget, _indent_level: usize) -> String {
    // Extract text content from props
    let content = widget
        .props
        .first()
        .map(|(_k, v)| generate_expression(v))
        .unwrap_or_else(|| "\"\"".to_string());

    format!("Text::new({}).render(),\n", content)
}

fn generate_button_widget(widget: &UIWidget, _indent_level: usize) -> String {
    // Extract label and on_click handler
    let label = widget
        .props
        .first()
        .map(|(_k, v)| generate_expression(v))
        .unwrap_or_else(|| "\"Button\"".to_string());

    let mut code = format!("Button::new({})", label);

    // Look for on_click handler in props
    for (key, _value) in &widget.props {
        if key == "on_click" {
            code.push_str("\n    .on_click(move || { /* handler */ })");
        }
    }

    code.push_str(".render(),\n");
    code
}

fn generate_text_input_widget(_widget: &UIWidget, _indent_level: usize) -> String {
    "/* TextInput widget - TODO */\nVNode::Empty,\n".to_string()
}

fn generate_checkbox_widget(_widget: &UIWidget, _indent_level: usize) -> String {
    "/* Checkbox widget - TODO */\nVNode::Empty,\n".to_string()
}

fn generate_slider_widget(_widget: &UIWidget, _indent_level: usize) -> String {
    "/* Slider widget - TODO */\nVNode::Empty,\n".to_string()
}

fn generate_color_picker_widget(_widget: &UIWidget, _indent_level: usize) -> String {
    "/* ColorPicker widget - TODO */\nVNode::Empty,\n".to_string()
}

fn generate_vstack_widget(widget: &UIWidget, indent_level: usize) -> String {
    let mut code = String::from("Flex::new()\n");
    code.push_str("    .direction(FlexDirection::Column)\n");
    code.push_str("    .children(vec![\n");

    for child in &widget.children {
        code.push_str("        ");
        code.push_str(generate_ui_node(child, indent_level + 1).trim_start());
    }

    code.push_str("    ])\n");
    code.push_str("    .render(),\n");
    code
}

fn generate_hstack_widget(widget: &UIWidget, indent_level: usize) -> String {
    let mut code = String::from("Flex::new()\n");
    code.push_str("    .direction(FlexDirection::Row)\n");
    code.push_str("    .children(vec![\n");

    for child in &widget.children {
        code.push_str("        ");
        code.push_str(generate_ui_node(child, indent_level + 1).trim_start());
    }

    code.push_str("    ])\n");
    code.push_str("    .render(),\n");
    code
}

fn generate_scroll_area_widget(widget: &UIWidget, indent_level: usize) -> String {
    // ScrollArea could be implemented as a Container with overflow:scroll
    let mut code = String::from("/* ScrollArea */\n");
    code.push_str("Flex::new().direction(FlexDirection::Column).children(vec![\n");

    for child in &widget.children {
        code.push_str("    ");
        code.push_str(generate_ui_node(child, indent_level + 1).trim_start());
    }

    code.push_str("]).render(),\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Decorator, Expression, Statement, Type};

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
    fn test_generate_component_struct_no_signals() {
        let func = create_test_component("Counter", vec![]);
        let code = generate_component_struct(&func);

        assert!(code.contains("pub struct Counter"));
    }

    #[test]
    fn test_generate_component_struct_with_signal() {
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
                            value: crate::parser::ast::Literal::Int(0),
                            location: None,
                        },
                    )],
                    location: None,
                },
                location: None,
            }],
        );

        let code = generate_component_struct(&func);

        assert!(code.contains("count: Signal<i32>"));
    }

    #[test]
    fn test_generate_constructor() {
        let func = create_test_component("Counter", vec![]);
        let code = generate_constructor(&func);

        assert!(code.contains("impl Counter"));
        assert!(code.contains("pub fn new() -> Self"));
    }

    #[test]
    fn test_generate_component_impl() {
        let func = create_test_component("Counter", vec![]);
        let code = generate_component_impl(&func);

        assert!(code.contains("impl Component for Counter"));
        assert!(code.contains("fn render(&self) -> VNode"));
    }

    #[test]
    fn test_generate_text_widget() {
        let widget = UIWidget {
            name: "Text".to_string(),
            props: vec![],
            children: vec![],
        };

        let code = generate_text_widget(&widget, 1);
        assert!(code.contains("Text::new"));
        assert!(code.contains(".render()"));
    }

    #[test]
    fn test_generate_button_widget() {
        let widget = UIWidget {
            name: "Button".to_string(),
            props: vec![],
            children: vec![],
        };

        let code = generate_button_widget(&widget, 1);
        assert!(code.contains("Button::new"));
        assert!(code.contains(".render()"));
    }

    #[test]
    fn test_generate_vstack_widget() {
        let widget = UIWidget {
            name: "VStack".to_string(),
            props: vec![],
            children: vec![UINode::Widget(UIWidget {
                name: "Text".to_string(),
                props: vec![],
                children: vec![],
            })],
        };

        let code = generate_vstack_widget(&widget, 1);
        assert!(code.contains("Flex::new()"));
        assert!(code.contains("FlexDirection::Column"));
    }

    #[test]
    fn test_generate_hstack_widget() {
        let widget = UIWidget {
            name: "HStack".to_string(),
            props: vec![],
            children: vec![],
        };

        let code = generate_hstack_widget(&widget, 1);
        assert!(code.contains("Flex::new()"));
        assert!(code.contains("FlexDirection::Row"));
    }

    #[test]
    fn test_generate_scroll_area_widget() {
        let widget = UIWidget {
            name: "ScrollArea".to_string(),
            props: vec![],
            children: vec![],
        };

        let code = generate_scroll_area_widget(&widget, 1);
        assert!(code.contains("Flex::new()"));
    }

    #[test]
    fn test_generate_slider_widget() {
        let widget = UIWidget {
            name: "Slider".to_string(),
            props: vec![],
            children: vec![],
        };

        let code = generate_slider_widget(&widget, 1);
        assert!(code.contains("Slider"));
    }

    #[test]
    fn test_generate_color_picker_widget() {
        let widget = UIWidget {
            name: "ColorPicker".to_string(),
            props: vec![],
            children: vec![],
        };

        let code = generate_color_picker_widget(&widget, 1);
        assert!(code.contains("ColorPicker"));
    }

    #[test]
    fn test_generate_component_complete() {
        let func = create_test_component("Counter", vec![]);
        let result = generate_component(&func);

        assert!(result.is_ok());
        let code = result.unwrap();

        // Should contain imports, struct, constructor, and Component impl
        assert!(code.contains("use windjammer_ui::Signal"));
        assert!(code.contains("pub struct Counter"));
        assert!(code.contains("impl Counter"));
        assert!(code.contains("impl Component for Counter"));
    }

    #[test]
    fn test_generate_imports() {
        let imports = generate_imports();
        assert!(imports.contains("use windjammer_ui::Signal"));
        assert!(imports.contains("use windjammer_ui::component::Component"));
        assert!(imports.contains("use windjammer_ui::components::button::Button"));
    }
}
