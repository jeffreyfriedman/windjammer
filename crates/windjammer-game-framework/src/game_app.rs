//! Complete game application with integrated systems

use crate::audio::AudioSystem;
use crate::ecs::World;
use crate::input::Input;
use crate::math::Vec2;
use crate::physics::PhysicsWorld;
use crate::rendering::{Camera, RenderContext};
use crate::time::Time;

/// Complete game application
pub struct GameApp {
    pub world: World,
    pub physics: PhysicsWorld,
    pub audio: AudioSystem,
    pub input: Input,
    pub time: Time,
    pub camera: Camera,
    running: bool,
}

impl GameApp {
    /// Create a new game application
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            world: World::new(),
            physics: PhysicsWorld::new(Vec2::new(0.0, -9.81)),
            audio: AudioSystem::new()?,
            input: Input::new(),
            time: Time::new(),
            camera: Camera::new(),
            running: true,
        })
    }

    /// Initialize the game
    pub fn init(&mut self) {
        // Override in game implementation
    }

    /// Update game logic
    pub fn update(&mut self, _dt: f32) {
        // Update time
        self.time.update();

        // Update physics
        self.physics.step();

        // Note: World doesn't have an update method - systems should be run manually
        // or through a system scheduler

        // Clear input state for next frame
        self.input.clear_frame_state();
    }

    /// Render the game
    pub fn render(&mut self, _render_context: &mut RenderContext) {
        // Override in game implementation
    }

    /// Check if game is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the game
    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Default for GameApp {
    fn default() -> Self {
        Self::new().expect("Failed to create game app")
    }
}

/// Game loop trait for custom games
pub trait Game {
    /// Initialize game
    fn init(&mut self, app: &mut GameApp);

    /// Update game logic
    fn update(&mut self, app: &mut GameApp, dt: f32);

    /// Render game
    fn render(&mut self, app: &mut GameApp, render_context: &mut RenderContext);

    /// Handle input
    fn handle_input(&mut self, app: &mut GameApp);
}

/// Run a game with the given implementation
pub fn run_game<G: Game>(mut game: G) -> Result<(), String> {
    let mut app = GameApp::new()?;

    game.init(&mut app);

    // Simple game loop (in real implementation, this would be driven by winit event loop)
    let target_fps = 60.0;
    let dt = 1.0 / target_fps;

    while app.is_running() {
        game.handle_input(&mut app);
        game.update(&mut app, dt);

        let mut render_context = RenderContext::new();
        game.render(&mut app, &mut render_context);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_app_creation() {
        let app = GameApp::new();
        assert!(app.is_ok());
    }

    #[test]
    fn test_game_app_update() {
        let mut app = GameApp::new().unwrap();
        app.update(0.016);
        assert!(app.is_running());
    }

    #[test]
    fn test_game_app_stop() {
        let mut app = GameApp::new().unwrap();
        assert!(app.is_running());
        app.stop();
        assert!(!app.is_running());
    }

    // Note: run_game() is not tested here as it contains an infinite loop
    // Real games will break the loop based on window events or user input
}
