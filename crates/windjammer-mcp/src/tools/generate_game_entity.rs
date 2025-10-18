//! Tool for generating game entities

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateGameEntityArgs {
    pub name: String,
    pub entity_type: String,
    pub include_physics: Option<bool>,
    pub include_health: Option<bool>,
}

pub fn execute(args: Value) -> Result<Vec<Value>, String> {
    let parsed_args: GenerateGameEntityArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let entity_name = &parsed_args.name;
    let _entity_type = &parsed_args.entity_type;
    let include_physics = parsed_args.include_physics.unwrap_or(false);
    let include_health = parsed_args.include_health.unwrap_or(false);

    let mut fields = vec![
        "    position: Vec2,".to_string(),
        "    rotation: f32,".to_string(),
        "    scale: Vec2,".to_string(),
    ];
    let new_params = ["pos: Vec2".to_string()];
    let mut new_fields = vec!["position: pos".to_string()];

    if include_physics {
        fields.push("    velocity: Vec2,".to_string());
        fields.push("    acceleration: Vec2,".to_string());
        new_fields.push("velocity: Vec2 { x: 0.0, y: 0.0 }".to_string());
        new_fields.push("acceleration: Vec2 { x: 0.0, y: 0.0 }".to_string());
    }
    if include_health {
        fields.push("    health: int,".to_string());
        fields.push("    max_health: int,".to_string());
        new_fields.push("health: 100".to_string());
        new_fields.push("max_health: 100".to_string());
    }

    let update_method = if include_physics {
        r#"
    fn update(delta: f32) {
        velocity += acceleration * delta
        position += velocity * delta
        // Basic friction/drag
        velocity *= 0.95
    }
"#
        .to_string()
    } else {
        String::new()
    };

    let render_method = r#"
    fn render(ctx: RenderContext) {
        ctx.draw_rect(position.x, position.y, scale.x, scale.y, Color.BLUE)
        // Add more sophisticated rendering based on entity_type
    }
"#
    .to_string();

    let entity_code = format!(
        r#"use windjammer_ui.game.*

@game_entity
struct {} {{
{}
}}

impl {} {{
    fn new({}) -> Self {{
        Self {{
            {},
            rotation: 0.0,
            scale: Vec2 {{ x: 32.0, y: 32.0 }},
        }}
    }}

    {}
    {}
}}
"#,
        entity_name,
        fields.join("\n"),
        entity_name,
        new_params.join(", "),
        new_fields.join(",\n            "),
        update_method,
        render_method
    );

    Ok(vec![json!({
        "text": entity_code,
        "language": "windjammer"
    })])
}
