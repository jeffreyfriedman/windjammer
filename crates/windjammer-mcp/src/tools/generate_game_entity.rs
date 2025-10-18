//! Tool for generating game entities

use crate::protocol::{Tool, ToolDefinition, ToolResult};
use serde_json::{json, Value};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "generate_game_entity".to_string(),
        description:
            "Generate a game entity with @game decorator, ECS components, and game loop methods"
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Entity name (PascalCase)"
                },
                "entity_type": {
                    "type": "string",
                    "description": "Type of game entity",
                    "enum": ["player", "enemy", "projectile", "item", "npc", "custom"]
                },
                "components": {
                    "type": "array",
                    "description": "ECS components to include",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "type": { "type": "string" }
                        },
                        "required": ["name", "type"]
                    }
                },
                "include_physics": {
                    "type": "boolean",
                    "description": "Include physics components (velocity, acceleration)"
                },
                "include_health": {
                    "type": "boolean",
                    "description": "Include health/damage system"
                }
            },
            "required": ["name", "entity_type"]
        }),
    }
}

pub fn execute(arguments: Value) -> ToolResult {
    let name = arguments["name"]
        .as_str()
        .ok_or("Missing 'name' parameter")?;

    let entity_type = arguments["entity_type"].as_str().unwrap_or("custom");

    let include_physics = arguments["include_physics"].as_bool().unwrap_or(true);
    let include_health = arguments["include_health"].as_bool().unwrap_or(false);
    let custom_components = arguments["components"].as_array();

    // Generate game entity code
    let mut code = String::from("use windjammer_ui.game.*\n\n");

    // Add entity struct with @game decorator
    code.push_str(&format!(
        "@derive(Debug, Clone)\n@game\nstruct {} {{\n",
        name
    ));

    // Always include position
    code.push_str("    position: Vec2\n");

    // Add physics components if requested
    if include_physics {
        code.push_str("    velocity: Vec2\n");
        code.push_str("    acceleration: Vec2\n");
    }

    // Add health if requested
    if include_health {
        code.push_str("    health: int\n");
        code.push_str("    max_health: int\n");
    }

    // Add entity-type specific components
    match entity_type {
        "player" => {
            code.push_str("    speed: f32\n");
            code.push_str("    score: int\n");
        }
        "enemy" => {
            code.push_str("    patrol_path: [Vec2]\n");
            code.push_str("    aggro_range: f32\n");
        }
        "projectile" => {
            code.push_str("    damage: int\n");
            code.push_str("    lifetime: f32\n");
        }
        "item" => {
            code.push_str("    item_type: string\n");
            code.push_str("    pickable: bool\n");
        }
        "npc" => {
            code.push_str("    dialogue: [string]\n");
            code.push_str("    interaction_radius: f32\n");
        }
        _ => {}
    }

    // Add custom components
    if let Some(components) = custom_components {
        for component in components {
            let comp_name = component["name"].as_str().unwrap_or("field");
            let comp_type = component["type"].as_str().unwrap_or("int");
            code.push_str(&format!("    {}: {}\n", comp_name, comp_type));
        }
    }

    code.push_str("}\n\n");

    // Add impl block with common methods
    code.push_str(&format!("impl {} {{\n", name));

    // Constructor
    code.push_str("    fn new(pos: Vec2) -> Self {\n");
    code.push_str(&format!("        {} {{\n", name));
    code.push_str("            position: pos,\n");

    if include_physics {
        code.push_str("            velocity: Vec2 { x: 0.0, y: 0.0 },\n");
        code.push_str("            acceleration: Vec2 { x: 0.0, y: 0.0 },\n");
    }

    if include_health {
        code.push_str("            health: 100,\n");
        code.push_str("            max_health: 100,\n");
    }

    // Add entity-specific defaults
    match entity_type {
        "player" => {
            code.push_str("            speed: 5.0,\n");
            code.push_str("            score: 0,\n");
        }
        "enemy" => {
            code.push_str("            patrol_path: [],\n");
            code.push_str("            aggro_range: 10.0,\n");
        }
        "projectile" => {
            code.push_str("            damage: 10,\n");
            code.push_str("            lifetime: 2.0,\n");
        }
        "item" => {
            code.push_str("            item_type: \"generic\",\n");
            code.push_str("            pickable: true,\n");
        }
        "npc" => {
            code.push_str("            dialogue: [\"Hello!\"],\n");
            code.push_str("            interaction_radius: 2.0,\n");
        }
        _ => {}
    }

    code.push_str("        }\n    }\n\n");

    // Update method
    code.push_str("    fn update(delta: f32) {\n");
    if include_physics {
        code.push_str("        velocity += acceleration * delta\n");
        code.push_str("        position += velocity * delta\n");
    }
    code.push_str("    }\n\n");

    // Render method
    code.push_str("    fn render(ctx: RenderContext) {\n");
    code.push_str("        ctx.draw_rect(position.x, position.y, 32, 32, Color.WHITE)\n");
    code.push_str("    }\n");

    code.push_str("}\n");

    Ok(vec![json!({
        "type": "text",
        "text": code
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_player_entity() {
        let args = json!({
            "name": "Player",
            "entity_type": "player",
            "include_physics": true,
            "include_health": true
        });

        let result = execute(args);
        assert!(result.is_ok());

        let content = result.unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("@game"));
        assert!(text.contains("struct Player"));
        assert!(text.contains("position: Vec2"));
        assert!(text.contains("velocity: Vec2"));
        assert!(text.contains("health: int"));
    }

    #[test]
    fn test_generate_enemy_entity() {
        let args = json!({
            "name": "Zombie",
            "entity_type": "enemy",
            "include_physics": false,
            "include_health": true
        });

        let result = execute(args);
        assert!(result.is_ok());

        let content = result.unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("patrol_path"));
        assert!(text.contains("aggro_range"));
    }
}
