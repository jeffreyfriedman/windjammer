//! # Windjammer Game Engine
//!
//! A high-performance 2D/3D game engine for Windjammer with support for:
//! - **Graphics**: Metal, Vulkan, DirectX 12, WebGPU (via wgpu)
//! - **Physics**: 2D and 3D physics (via rapier)
//! - **Audio**: Multiple audio backends (rodio, kira)
//! - **Cross-platform**: Desktop (Windows, macOS, Linux), Web (WASM)
//!
//! ## Philosophy
//!
//! - **Performance**: Zero-cost abstractions, SIMD-optimized math
//! - **Simplicity**: Clean API inspired by Unity/Godot but with Rust safety
//! - **Flexibility**: Modular architecture, choose what you need
//!
//! ## Example
//!
//! ```ignore
//! use windjammer_game::prelude::*;
//!
//! @game
//! struct MyGame {
//!     player: Entity,
//!     enemies: Vec<Entity>,
//! }
//!
//! impl GameLoop for MyGame {
//!     fn update(&mut self, delta: f32) {
//!         // Update game logic
//!     }
//!     
//!     fn render(&mut self, ctx: &mut RenderContext) {
//!         // Render game
//!     }
//! }
//! ```

pub mod assets; // Asset loading and management
pub mod audio; // Audio playback
pub mod ecs; // Entity-Component-System
pub mod game_loop; // Game loop with fixed timestep
pub mod input; // Input handling
pub mod math; // Math types (Vec2, Vec3, Mat4, etc.)
pub mod physics; // Physics integration
pub mod rendering; // Graphics rendering
pub mod time; // Time and delta time management
pub mod window; // Window creation and management

/// Prelude module with commonly used types and traits
pub mod prelude {
    pub use crate::assets::{AssetManager, Handle};
    pub use crate::audio::{AudioPlayer, AudioSource};
    pub use crate::ecs::{Component, Entity, System, World};
    pub use crate::input::{Input, KeyCode, MouseButton};
    pub use crate::math::{Mat4, Quat, Vec2, Vec3, Vec4};
    pub use crate::physics::{Collider, PhysicsWorld, RigidBody};
    pub use crate::rendering::{Camera, Material, Mesh, RenderContext, Sprite};
    pub use crate::time::Time;
    pub use crate::window::{Window, WindowConfig};

    // Re-export the game loop trait
    pub use crate::GameLoop;
}

/// Main game loop trait
pub trait GameLoop {
    /// Initialize the game
    fn init(&mut self) {}

    /// Update game logic (called every frame)
    fn update(&mut self, delta: f32);

    /// Render the game (called every frame)
    fn render(&mut self, ctx: &mut rendering::RenderContext);

    /// Handle input events
    fn handle_input(&mut self, input: &input::Input) {
        let _ = input; // Default: do nothing
    }

    /// Cleanup when game exits
    fn cleanup(&mut self) {}
}

/// Run a game with default configuration
pub fn run<G: GameLoop + 'static>(game: G) -> Result<(), String> {
    game_loop::run_game_loop(game, game_loop::GameLoopConfig::default())
}

/// Run a game with custom configuration
pub fn run_with_config<G: GameLoop + 'static>(
    game: G,
    config: game_loop::GameLoopConfig,
) -> Result<(), String> {
    game_loop::run_game_loop(game, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestGame;

    impl GameLoop for TestGame {
        fn update(&mut self, _delta: f32) {}
        fn render(&mut self, _ctx: &mut rendering::RenderContext) {}
    }

    #[test]
    fn test_game_creation() {
        let game = TestGame;
        assert!(run(game).is_ok());
    }
}
