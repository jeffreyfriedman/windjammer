//! Tool for generating UI components

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateComponentArgs {
    pub name: String,
    pub props: Option<Vec<ComponentProp>>,
    pub render_template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentProp {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub default_value: Option<String>,
}

pub fn execute(args: Value) -> Result<Vec<Value>, String> {
    let parsed_args: GenerateComponentArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let component_name = &parsed_args.name;
    let props = parsed_args.props.unwrap_or_default();
    let render_template = parsed_args.render_template.as_deref().unwrap_or("div");

    let prop_struct_fields: Vec<String> = props
        .iter()
        .map(|p| {
            let default = p
                .default_value
                .as_ref()
                .map(|d| format!(" = {}", d))
                .unwrap_or_default();
            format!("    pub {}: {}{}", p.name, p.prop_type, default)
        })
        .collect();

    let prop_destructuring: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
    let prop_destructuring_str = if prop_destructuring.is_empty() {
        String::new()
    } else {
        format!("let {{ {} }} = self;", prop_destructuring.join(", "))
    };

    let template_content = match render_template {
        "div" => format!(
            r#"        VElement::new("div")
            .attr("class", "{}_container")
            .child(VNode::Text(VText::new("Hello, {}!")))
            .into()"#,
            component_name.to_lowercase(),
            component_name
        ),
        "button" => format!(
            r#"        VElement::new("button")
            .attr("onclick", "self.on_click")
            .child(VNode::Text(VText::new("Click me, {}!")))
            .into()"#,
            component_name
        ),
        "form" => r#"        VElement::new("form")
            .child(VNode::Element(VElement::new("input").attr("type", "text").into()))
            .child(VNode::Element(VElement::new("button").child(VNode::Text(VText::new("Submit"))).into()))
            .into()"#
            .to_string(),
        "list" => r#"        VElement::new("ul")
            .child(VNode::Text(VText::new("Item 1")))
            .child(VNode::Text(VText::new("Item 2")))
            .into()"#
            .to_string(),
        "card" => format!(
            r#"        VElement::new("div")
            .attr("class", "card")
            .child(VNode::Element(VElement::new("h2").child(VNode::Text(VText::new("{} Card"))).into()))
            .child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new("This is a generic card component."))).into()))
            .into()"#,
            component_name
        ),
        _ => format!(
            r#"        VElement::new("div")
            .child(VNode::Text(VText::new("Custom component: {}"))).into()"#,
            component_name
        ),
    };

    let component_code = format!(
        r#"use windjammer_ui.prelude.*
use windjammer_ui.vdom.{{VElement, VNode, VText}}

@component
struct {} {{
{}
}}

impl {} {{
    fn new({}) -> Self {{
        Self {{ {} }}
    }}

    fn render() -> VNode {{
        {}
        {}
    }}
}}
"#,
        component_name,
        prop_struct_fields.join("\n"),
        component_name,
        props
            .iter()
            .map(|p| format!("{}: {}", p.name, p.prop_type))
            .collect::<Vec<String>>()
            .join(", "),
        props
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<String>>()
            .join(", "),
        prop_destructuring_str,
        template_content
    );

    Ok(vec![json!({
        "text": component_code,
        "language": "windjammer"
    })])
}
