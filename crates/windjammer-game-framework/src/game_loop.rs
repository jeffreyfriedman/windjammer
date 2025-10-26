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
    /// Enable VSync (limits FPS to monitor refresh rate)
    pub vsync: bool,
    /// Maximum frame time (prevents spiral of death)
    pub max_frame_time: f32,
}

impl Default for GameLoopConfig {
    fn default() -> Self {
        Self {
            target_ups: 60,
            max_frame_skip: 5,
            vsync: true,
            max_frame_time: 0.25,
        }
    }
}

impl GameLoopConfig {
    /// Create a new configuration with custom update rate
    pub fn with_ups(ups: u32) -> Self {
        Self {
            target_ups: ups,
            ..Default::default()
        }
    }

    /// Set maximum frame skip
    pub fn with_max_frame_skip(mut self, max_skip: u32) -> Self {
        self.max_frame_skip = max_skip;
        self
    }

    /// Set VSync
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }
}

/// Game loop runner with state management
pub struct GameLoopRunner {
    time: Time,
    input: Input,
    render_ctx: RenderContext,
    config: GameLoopConfig,
    accumulator: Duration,
    last_time: Instant,
    running: bool,
}

impl GameLoopRunner {
    /// Create a new game loop runner
    pub fn new(config: GameLoopConfig) -> Self {
        Self {
            time: Time::new(),
            input: Input::new(),
            render_ctx: RenderContext::new(),
            config,
            accumulator: Duration::ZERO,
            last_time: Instant::now(),
            running: false,
        }
    }

    /// Get the current time
    pub fn time(&self) -> &Time {
        &self.time
    }

    /// Get the input state
    pub fn input(&self) -> &Input {
        &self.input
    }

    /// Get mutable input state
    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }

    /// Check if the loop is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the game loop
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Run one frame of the game loop
    pub fn tick<G: GameLoop>(&mut self, game: &mut G) -> bool {
        let current_time = Instant::now();
        let frame_time = current_time - self.last_time;
        self.last_time = current_time;

        // Cap frame time to prevent spiral of death
        let frame_time = frame_time.min(Duration::from_secs_f32(self.config.max_frame_time));
        self.accumulator += frame_time;

        let fixed_dt = 1.0 / self.config.target_ups as f32;
        let fixed_dt_duration = Duration::from_secs_f32(fixed_dt);

        // Fixed timestep updates
        let mut updates = 0;
        while self.accumulator >= fixed_dt_duration && updates < self.config.max_frame_skip {
            game.handle_input(&self.input);
            game.update(fixed_dt);

            self.accumulator -= fixed_dt_duration;
            updates += 1;
        }

        // Render
        game.render(&mut self.render_ctx);

        // Update time
        self.time.update();

        // Clear frame-specific input state
        self.input.clear_frame_state();

        self.running
    }
}

/// Run a game with a fixed timestep game loop
///
/// This is a simple headless runner for testing. For production games,
/// use `GameLoopRunner` with a windowing system like `winit`.
pub fn run_game_loop<G: GameLoop>(mut game: G, config: GameLoopConfig) -> Result<(), String> {
    game.init();

    let mut runner = GameLoopRunner::new(config);
    runner.running = true;

    // Simple loop for now (in production, this would integrate with winit event loop)
    let mut frame_count = 0;
    let max_frames = 60; // Run for 60 frames in headless mode

    while frame_count < max_frames && runner.is_running() {
        runner.tick(&mut game);
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
        input_handled: bool,
    }

    impl GameLoop for TestGame {
        fn update(&mut self, _delta: f32) {
            self.update_count += 1;
        }

        fn render(&mut self, _ctx: &mut RenderContext) {
            self.render_count += 1;
        }

        fn handle_input(&mut self, _input: &Input) {
            self.input_handled = true;
        }
    }

    #[test]
    fn test_game_loop_config_default() {
        let config = GameLoopConfig::default();
        assert_eq!(config.target_ups, 60);
        assert_eq!(config.max_frame_skip, 5);
        assert!(config.vsync);
        assert_eq!(config.max_frame_time, 0.25);
    }

    #[test]
    fn test_game_loop_config_builder() {
        let config = GameLoopConfig::with_ups(120)
            .with_max_frame_skip(10)
            .with_vsync(false);

        assert_eq!(config.target_ups, 120);
        assert_eq!(config.max_frame_skip, 10);
        assert!(!config.vsync);
    }

    #[test]
    fn test_game_loop_runner_creation() {
        let runner = GameLoopRunner::new(GameLoopConfig::default());
        assert!(!runner.is_running());
        assert_eq!(runner.time().frame_count(), 0);
    }

    #[test]
    fn test_game_loop_runner_tick() {
        let mut runner = GameLoopRunner::new(GameLoopConfig::default());
        runner.running = true;

        let mut game = TestGame {
            update_count: 0,
            render_count: 0,
            input_handled: false,
        };

        // Simulate some time passing to ensure accumulator has enough time for an update
        std::thread::sleep(std::time::Duration::from_millis(20));

        runner.tick(&mut game);

        // Game should have been updated and rendered
        assert!(game.update_count > 0);
        assert_eq!(game.render_count, 1);
        assert!(game.input_handled);
    }

    #[test]
    fn test_game_loop_runner_stop() {
        let mut runner = GameLoopRunner::new(GameLoopConfig::default());
        runner.running = true;
        assert!(runner.is_running());

        runner.stop();
        assert!(!runner.is_running());
    }

    #[test]
    fn test_game_loop_runner_input_access() {
        let mut runner = GameLoopRunner::new(GameLoopConfig::default());

        // Test immutable access
        let input = runner.input();
        assert!(!input.is_key_pressed(crate::input::KeyCode::Space));

        // Test mutable access
        let input_mut = runner.input_mut();
        input_mut.press_key(crate::input::KeyCode::Space);
        assert!(runner.input().is_key_pressed(crate::input::KeyCode::Space));
    }

    #[test]
    fn test_game_loop_runs() {
        let game = TestGame {
            update_count: 0,
            render_count: 0,
            input_handled: false,
        };

        let result = run_game_loop(game, GameLoopConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_game_loop_counts() {
        let mut game = TestGame {
            update_count: 0,
            render_count: 0,
            input_handled: false,
        };

        let mut runner = GameLoopRunner::new(GameLoopConfig::default());
        runner.running = true;

        // Run 10 ticks with sufficient sleep to ensure accumulator fills
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(20)); // Sleep before tick to ensure time accumulates
            runner.tick(&mut game);
        }

        // Should have updated and rendered multiple times
        assert!(
            game.update_count >= 10,
            "Expected at least 10 updates, got {}",
            game.update_count
        );
        assert_eq!(game.render_count, 10);
    }
}
