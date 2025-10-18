//! Tool for generating UI components

use crate::protocol::{Tool, ToolDefinition, ToolResult};
use serde_json::{json, Value};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "generate_component".to_string(),
        description:
            "Generate a Windjammer UI component with @component decorator, props, and render method"
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Component name (PascalCase)"
                },
                "props": {
                    "type": "array",
                    "description": "Component properties",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "type": { "type": "string" },
                            "default": { "type": "string" }
                        },
                        "required": ["name", "type"]
                    }
                },
                "render_template": {
                    "type": "string",
                    "description": "Template for render method (div, button, form, etc.)",
                    "enum": ["div", "button", "form", "list", "card", "custom"]
                }
            },
            "required": ["name"]
        }),
    }
}

pub fn execute(arguments: Value) -> ToolResult {
    let name = arguments["name"]
        .as_str()
        .ok_or("Missing 'name' parameter")?;

    let props = arguments["props"].as_array();
    let template = arguments["render_template"].as_str().unwrap_or("div");

    // Generate component code
    let mut code = format!(
        "use windjammer_ui.prelude.*\nuse windjammer_ui.vdom.{{VElement, VNode, VText}}\n\n"
    );

    // Add component struct with @component decorator
    code.push_str(&format!("@component\nstruct {} {{\n", name));

    if let Some(props_array) = props {
        for prop in props_array {
            let prop_name = prop["name"].as_str().unwrap_or("value");
            let prop_type = prop["type"].as_str().unwrap_or("string");
            code.push_str(&format!("    {}: {}\n", prop_name, prop_type));
        }
    } else {
        // Default prop
        code.push_str("    value: string\n");
    }

    code.push_str("}\n\n");

    // Add impl block with render method
    code.push_str(&format!("impl {} {{\n", name));
    code.push_str("    fn render() -> VNode {\n");

    // Generate render body based on template
    let render_body = match template {
        "button" => format!(
            "        VElement::new(\"button\")\n            .attr(\"class\", \"{}\")\n            .child(VNode::Text(VText::new(value)))\n            .into()",
            name.to_lowercase()
        ),
        "form" => format!(
            "        VElement::new(\"form\")\n            .attr(\"class\", \"{}\")\n            .child(VNode::Element(\n                VElement::new(\"input\")\n                    .attr(\"type\", \"text\")\n                    .attr(\"value\", value)\n            ))\n            .child(VNode::Element(\n                VElement::new(\"button\")\n                    .attr(\"type\", \"submit\")\n                    .child(VNode::Text(VText::new(\"Submit\")))\n            ))\n            .into()",
            name.to_lowercase()
        ),
        "list" => format!(
            "        VElement::new(\"ul\")\n            .attr(\"class\", \"{}\")\n            .child(VNode::Element(\n                VElement::new(\"li\")\n                    .child(VNode::Text(VText::new(value)))\n            ))\n            .into()",
            name.to_lowercase()
        ),
        "card" => format!(
            "        VElement::new(\"div\")\n            .attr(\"class\", \"{}-card\")\n            .child(VNode::Element(\n                VElement::new(\"h2\")\n                    .child(VNode::Text(VText::new(value)))\n            ))\n            .child(VNode::Element(\n                VElement::new(\"p\")\n                    .child(VNode::Text(VText::new(\"Card content\")))\n            ))\n            .into()",
            name.to_lowercase()
        ),
        _ => format!(
            "        VElement::new(\"div\")\n            .attr(\"class\", \"{}\")\n            .child(VNode::Text(VText::new(value)))\n            .into()",
            name.to_lowercase()
        ),
    };

    code.push_str(&render_body);
    code.push_str("\n    }\n}\n");

    Ok(vec![json!({
        "type": "text",
        "text": code
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_simple_component() {
        let args = json!({
            "name": "Button",
            "render_template": "button"
        });

        let result = execute(args);
        assert!(result.is_ok());

        let content = result.unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("@component"));
        assert!(text.contains("struct Button"));
        assert!(text.contains("fn render()"));
    }

    #[test]
    fn test_generate_component_with_props() {
        let args = json!({
            "name": "TextField",
            "props": [
                {"name": "label", "type": "string"},
                {"name": "value", "type": "string"},
                {"name": "disabled", "type": "bool"}
            ],
            "render_template": "form"
        });

        let result = execute(args);
        assert!(result.is_ok());

        let content = result.unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("label: string"));
        assert!(text.contains("value: string"));
        assert!(text.contains("disabled: bool"));
    }
}
