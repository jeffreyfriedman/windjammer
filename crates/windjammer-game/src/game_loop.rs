//! Game loop implementation with fixed timestep

use crate::input::Input;
use crate::rendering::RenderContext;
use crate::time::Time;
use crate::GameLoop;
use std::time::{Duration, Instant};

/// Game loop configuration
pub struct GameLoopConfig {
    /// Target updates per second (fixed timestep)
    pub target_ups: u32,
    /// Maximum frame skip to prevent spiral of death
    pub max_frame_skip: u32,
}

impl Default for GameLoopConfig {
    fn default() -> Self {
        Self {
            target_ups: 60,
            max_frame_skip: 5,
        }
    }
}

/// Run a game with a fixed timestep game loop
pub fn run_game_loop<G: GameLoop>(mut game: G, config: GameLoopConfig) -> Result<(), String> {
    game.init();

    let mut time = Time::new();
    let mut input = Input::new();
    let mut render_ctx = RenderContext::new();

    let fixed_dt = 1.0 / config.target_ups as f32;
    let fixed_dt_duration = Duration::from_secs_f32(fixed_dt);

    let mut accumulator = Duration::ZERO;
    let mut last_time = Instant::now();

    // Simple loop for now (in production, this would integrate with winit event loop)
    let mut frame_count = 0;
    let max_frames = 60; // Run for 60 frames in headless mode

    while frame_count < max_frames {
        let current_time = Instant::now();
        let frame_time = current_time - last_time;
        last_time = current_time;

        // Cap frame time to prevent spiral of death
        let frame_time = frame_time.min(Duration::from_secs_f32(0.25));
        accumulator += frame_time;

        // Fixed timestep updates
        let mut updates = 0;
        while accumulator >= fixed_dt_duration && updates < config.max_frame_skip {
            game.handle_input(&input);
            game.update(fixed_dt);

            accumulator -= fixed_dt_duration;
            updates += 1;
        }

        // Render
        game.render(&mut render_ctx);

        // Update time
        time.update();

        // Clear frame-specific input state
        input.clear_frame_state();

        frame_count += 1;

        // Sleep to maintain target frame rate (simple approach)
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    game.cleanup();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestGame {
        update_count: u32,
        render_count: u32,
    }

    impl GameLoop for TestGame {
        fn update(&mut self, _delta: f32) {
            self.update_count += 1;
        }

        fn render(&mut self, _ctx: &mut RenderContext) {
            self.render_count += 1;
        }
    }

    #[test]
    fn test_game_loop_config() {
        let config = GameLoopConfig::default();
        assert_eq!(config.target_ups, 60);
        assert_eq!(config.max_frame_skip, 5);
    }

    #[test]
    fn test_game_loop_runs() {
        let game = TestGame {
            update_count: 0,
            render_count: 0,
        };

        let result = run_game_loop(game, GameLoopConfig::default());
        assert!(result.is_ok());
    }
}
