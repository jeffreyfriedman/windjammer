//! Game framework module for Windjammer UI
//!
//! Build cross-platform 2D/3D games using the same reactive component model.

pub mod ecs;
pub mod input;
pub mod math;
pub mod physics;
pub mod render;
pub mod time;

pub use ecs::{Entity, EntityId, GameEntity, World};
pub use input::{Input, Key, MouseButton as GameMouseButton};
pub use math::{Vec2, Vec3};
pub use render::{Color, RenderContext, Sprite};
pub use time::Time;

/// Trait for game loop implementation
pub trait GameLoop: Send + Sync {
    /// Update game state (fixed timestep, typically 60 FPS)
    fn update(&mut self, delta: f32);

    /// Render the game
    fn render(&self, ctx: &RenderContext);

    /// Called when the game starts
    fn start(&mut self) {}

    /// Called when the game ends
    fn cleanup(&mut self) {}
}

/// Game configuration
pub struct GameConfig {
    /// Window title
    pub title: String,
    /// Window width
    pub width: u32,
    /// Window height
    pub height: u32,
    /// Target FPS
    pub target_fps: u32,
    /// Enable VSync
    pub vsync: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "Windjammer Game".to_string(),
            width: 800,
            height: 600,
            target_fps: 60,
            vsync: true,
        }
    }
}

/// Run a game with default configuration
pub fn run<G: GameLoop + 'static>(_game: G) -> Result<(), String> {
    run_with_config(_game, GameConfig::default())
}

/// Run a game with custom configuration
pub fn run_with_config<G: GameLoop + 'static>(
    mut _game: G,
    _config: GameConfig,
) -> Result<(), String> {
    // In a full implementation, this would:
    // 1. Initialize the platform-specific renderer
    // 2. Create the game loop with fixed timestep
    // 3. Handle input events
    // 4. Call update() and render() in a loop

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestGame {
        frame_count: u32,
    }

    impl GameLoop for TestGame {
        fn update(&mut self, _delta: f32) {
            self.frame_count += 1;
        }

        fn render(&self, _ctx: &RenderContext) {
            // Render game
        }
    }

    #[test]
    fn test_game_config_default() {
        let config = GameConfig::default();
        assert_eq!(config.title, "Windjammer Game");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.target_fps, 60);
        assert!(config.vsync);
    }

    #[test]
    fn test_game_loop() {
        let mut game = TestGame { frame_count: 0 };
        game.update(0.016); // ~60 FPS
        assert_eq!(game.frame_count, 1);
    }
}
