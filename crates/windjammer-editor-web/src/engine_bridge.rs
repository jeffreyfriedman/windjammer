//! Bridge between the Windjammer game engine and the web editor
//!
//! This module provides WASM bindings for running the game engine in the browser.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::{console, CanvasRenderingContext2d, HtmlCanvasElement};

/// Game engine instance that runs in the browser
#[wasm_bindgen]
pub struct GameEngine {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    running: bool,
    frame_count: u32,
}

#[wasm_bindgen]
impl GameEngine {
    /// Create a new game engine instance
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<GameEngine, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or("Could not get 2D context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        console::log_1(&"Game engine initialized!".into());

        Ok(GameEngine {
            canvas,
            context,
            running: false,
            frame_count: 0,
        })
    }

    /// Start the game engine
    pub fn start(&mut self) {
        self.running = true;
        console::log_1(&"Game engine started!".into());
    }

    /// Stop the game engine
    pub fn stop(&mut self) {
        self.running = false;
        console::log_1(&"Game engine stopped!".into());
    }

    /// Update the game engine (called every frame)
    pub fn update(&mut self, delta_time: f64) {
        if !self.running {
            return;
        }

        self.frame_count += 1;

        // Clear canvas
        let width = self.canvas.width() as f64;
        let height = self.canvas.height() as f64;
        self.context.clear_rect(0.0, 0.0, width, height);

        // Draw a simple animation to show the engine is running
        self.context.set_fill_style(&JsValue::from_str("#1a1a2e"));
        self.context.fill_rect(0.0, 0.0, width, height);

        // Draw a bouncing ball
        let t = (self.frame_count as f64 * 0.05).sin();
        let x = width / 2.0 + t * 100.0;
        let y = height / 2.0 + (self.frame_count as f64 * 0.03).cos() * 50.0;

        self.context.begin_path();
        self.context
            .arc(x, y, 20.0, 0.0, std::f64::consts::PI * 2.0)
            .unwrap();
        self.context.set_fill_style(&JsValue::from_str("#00d4ff"));
        self.context.fill();

        // Draw FPS counter
        self.context.set_fill_style(&JsValue::from_str("#ffffff"));
        self.context.set_font("14px monospace");
        let fps = (1000.0 / delta_time).round();
        let _ = self.context.fill_text(
            &format!("FPS: {} | Frame: {}", fps, self.frame_count),
            10.0,
            20.0,
        );
    }

    /// Get the current frame count
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Check if the engine is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Reset the engine
    pub fn reset(&mut self) {
        self.frame_count = 0;
        self.running = false;
        console::log_1(&"Game engine reset!".into());
    }
}

/// Scene representation for the editor
#[wasm_bindgen]
pub struct Scene {
    name: String,
    entities: Vec<Entity>,
}

#[wasm_bindgen]
impl Scene {
    /// Create a new empty scene
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> Scene {
        Scene {
            name,
            entities: Vec::new(),
        }
    }

    /// Get the scene name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Add an entity to the scene
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    /// Get the number of entities in the scene
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Serialize the scene to JSON
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.entities)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}

/// Entity representation for the editor
#[wasm_bindgen]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    id: u32,
    name: String,
    x: f64,
    y: f64,
    z: f64,
}

#[wasm_bindgen]
impl Entity {
    /// Create a new entity
    #[wasm_bindgen(constructor)]
    pub fn new(id: u32, name: String) -> Entity {
        Entity {
            id,
            name,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Get the entity ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the entity name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Set the entity position
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// Get the entity X position
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Get the entity Y position
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Get the entity Z position
    pub fn z(&self) -> f64 {
        self.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(1, "Test Entity".to_string());
        assert_eq!(entity.id(), 1);
        assert_eq!(entity.name(), "Test Entity");
    }

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new("Test Scene".to_string());
        assert_eq!(scene.name(), "Test Scene");
        assert_eq!(scene.entity_count(), 0);
    }
}

