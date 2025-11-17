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

pub mod ai_behavior_tree_simple; // Simple AI behavior tree (production-ready)
pub mod animation; // Animation system (skeletal, blending, IK)
pub mod animation_blending; // Advanced animation blending and crossfade
pub mod animation_controller; // High-level animation controller (state machine + blending)
pub mod animation_gpu; // GPU-accelerated skeletal animation
pub mod animation_ik; // Advanced IK system (FABRIK, Two-Bone, CCD, Look-At, Foot Placement)
pub mod animation_state_machine; // Animation state machine with transitions
pub mod assets; // Asset loading and management
pub mod audio; // Audio playback
pub mod audio_advanced; // Advanced audio (3D spatial, mixing, effects)
pub mod camera2d; // 2D camera system

#[cfg(feature = "3d")]
pub mod camera3d; // 3D camera system (perspective, third-person, first-person, free)

#[cfg(feature = "3d")]
pub mod character_controller; // 3D character controller (movement, jumping, cameras)

pub mod ecs; // Entity-Component-System (Rust implementation)
pub mod ecs_optimized; // Optimized ECS with archetype storage and query caching
pub mod ecs_windjammer; // Windjammer-friendly ECS API (recommended)
pub mod game_loop; // Game loop with fixed timestep
pub mod gamepad; // Gamepad/controller input
pub mod gltf_loader; // GLTF/GLB 3D model loader
pub mod input; // Input handling
pub mod lod; // Level of Detail (LOD) system
pub mod math; // Math types (Vec2, Vec3, Mat4, etc.)
pub mod mesh_clustering; // Mesh clustering system (Nanite-style)
pub mod navmesh; // Navigation mesh for 3D pathfinding
pub mod particles; // Particle system for visual effects
pub mod pathfinding; // A* pathfinding for AI navigation
pub mod pbr; // Physically-based rendering (PBR) materials and lighting
pub mod plugin;
pub mod plugin_ffi; // Plugin system for extensibility
pub mod post_processing; // Post-processing effects (bloom, DOF, etc.)
pub mod profiler; // Performance profiler for optimization
pub mod physics2d; // 2D physics system (Rapier2D integration with ECS)

#[cfg(feature = "3d")]
pub mod physics3d; // 3D physics system (Rapier3D integration with ECS)

pub mod renderer; // High-level 2D renderer (for Windjammer games)
pub mod renderer3d; // High-level 3D renderer (for Windjammer games)
pub mod terrain; // Heightmap-based terrain with LOD
pub mod rendering; // Graphics rendering
pub mod texture; // Texture loading and management
pub mod time; // Time and delta time management
pub mod transform; // 2D and 3D transform components
pub mod ui_immediate; // Immediate mode UI system (for games)
pub mod ui_system; // Retained mode UI system (for in-game UI)
pub mod weapon_system; // Weapon management (FPS/TPS/Action games)

#[cfg(not(target_arch = "wasm32"))]
pub mod window; // Window creation and management (native only)

/// Prelude module with commonly used types and traits
///
/// **Windjammer Philosophy**: This prelude exports only the Windjammer-friendly APIs
/// that hide Rust-specific concepts like lifetimes, trait bounds, and external crate types.
///
/// For advanced users who need the low-level Rust APIs, import them explicitly:
/// ```rust
/// use windjammer_game_framework::ecs::{World as RustWorld, ...};
/// ```
pub mod prelude {
    pub use crate::ai_behavior_tree_simple::{
        AIBlackboard, AIStatus, AITask, ConditionTask, SelectorTask, SequenceTask,
    }; // Simple AI behavior tree (production-ready)
    pub use crate::animation::{
        Animation, AnimationBlender, AnimationPlayer, Bone, IKChain, Skeleton, Transform,
    };
    pub use crate::animation_state_machine::{
        AnimationState, AnimationStateMachine, AnimationTransition, ConditionType, Parameter,
        TransitionCondition,
    }; // Animation state machine
    pub use crate::assets::{
        AssetManager, Handle, Texture, TextureConfig, TextureFilter, TextureFormat, TextureHandle,
        TextureLoader, TextureWrap,
    };
    pub use crate::audio::{AudioSystem, SpatialAudioSource};
    pub use crate::audio_advanced::{
        Audio3DParams, AudioBus, AudioClip, AudioEffect, AudioEngine, AudioSource, RolloffMode,
    }; // Advanced audio system
    pub use crate::camera2d::Camera2D;
    
    #[cfg(feature = "3d")]
    pub use crate::camera3d::{
        Camera3D, CameraProjection, FirstPersonCamera, FreeCamera, ThirdPersonCamera,
    }; // 3D camera system
    
    #[cfg(feature = "3d")]
    pub use crate::character_controller::{
        CharacterController, CharacterControllerSystem, CharacterMovementInput,
    }; // 3D character controller

    // Export ONLY Windjammer-friendly ECS API (zero crate leakage)
    pub use crate::ecs_windjammer::{Entity, System, World};

    pub use crate::game_loop::{run_game_loop, GameLoopConfig, GameLoopRunner};
    pub use crate::gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadManager, MAX_GAMEPADS}; // Gamepad/controller input
    pub use crate::gltf_loader::{
        AlphaMode, AnimationPath, FilterMode, GltfAnimation, GltfChannel, GltfDocument,
        GltfLoader, GltfMaterial, GltfMesh, GltfNode, GltfPrimitive, GltfSampler,
        GltfSampler2, GltfTexture, InterpolationMode, WrapMode,
    }; // GLTF/GLB 3D model loader
    pub use crate::input::{Input, Key, MouseButton}; // Ergonomic input API with held(), pressed(), released()
    pub use crate::lod::{LODConfig, LODLevel, LODSelector, LODStats}; // Level of Detail system
    pub use crate::math::{Mat4, Quat, Vec2, Vec3, Vec4};
    pub use crate::mesh_clustering::{
        ClusterConfig, ClusterStats, MeshCluster, MeshClusteringSystem,
    }; // Mesh clustering
    pub use crate::navmesh::{NavAgent, NavMesh, NavPath, NavPoly}; // Navigation mesh
    pub use crate::particles::{EmitterShape, Particle, ParticleEmitter, ParticleSystem}; // Particle system
    pub use crate::pathfinding::{GridPos, Heuristic, Path, PathfindingGrid}; // A* pathfinding
    pub use crate::pbr::{
        AlphaMode as PBRAlphaMode, DirectionalLight, EnvironmentMap, Light, PBRMaterial,
        PointLight, ShadowMap, SpotLight,
    }; // PBR materials and lighting
    pub use crate::plugin::{App, Plugin, PluginCategory, PluginDependency, PluginError, PluginManager, PluginState, Version, VersionReq};
    pub use crate::plugin_ffi::{DynamicPlugin, WjApp, WjPluginCategory, WjPluginDependency, WjPluginErrorCode, WjPluginInfo};
    pub use crate::post_processing::{
        BloomEffect, ChromaticAberrationEffect, ColorGrading, DepthOfFieldEffect, FilmGrainEffect,
        MotionBlurEffect, PostProcessing, ToneMappingMode, VignetteEffect,
    }; // Post-processing effects
    pub use crate::profiler::{
        FrameTimePercentiles, MemoryStats, PerformanceStats, ProfileGuard, ProfileScope, Profiler,
    }; // Performance profiler
    pub use crate::physics2d::{
        Collider2D, ColliderShape2D, PhysicsWorld2D, RigidBody2D, RigidBodyType,
    }; // 2D physics (Rapier2D integration)
    
    #[cfg(feature = "3d")]
    pub use crate::physics3d::{
        Collider3D, ColliderShape3D, PhysicsWorld3D, RaycastHit3D, RigidBody3D, RigidBodyType3D,
    }; // 3D physics (Rapier3D integration)
       // TODO v2.0: Re-enable physics_windjammer once fixed
       // pub use crate::physics_windjammer::{
       //     BodyHandle, BodyType, ColliderBuilder, ColliderHandle, CollisionShape, ConstraintHandle,
       //     ConstraintType, PhysicsMaterial, PhysicsWorldWj, RaycastHit, RigidBodyBuilder,
       // }; // Windjammer-friendly physics (zero crate leakage)
    pub use crate::renderer::{Color, Renderer}; // High-level 2D renderer (no wgpu types exposed)
    pub use crate::renderer3d::{Camera3D as RenderCamera3D, Renderer3D, SSGIConfig}; // High-level 3D renderer with SSGI (no wgpu types exposed)
    pub use crate::rendering::{Camera, Material, Mesh, RenderContext, Sprite, SpriteBatch};
    pub use crate::terrain::{Terrain, TerrainLOD, TerrainLayer, TerrainPatch}; // Heightmap terrain
    // Texture is now re-exported from assets module
    pub use crate::time::Time;
    pub use crate::transform::{Transform2D, Transform3D};
    pub use crate::ui_immediate::{DrawCommand, LayoutDirection, UIStyle, UI}; // Immediate mode UI
    pub use crate::ui_system::{
        LayoutType, UIElement, UIElementType, UIEvent, UIStyle as UIStyleRetained, UISystem,
    }; // Retained mode UI
    pub use crate::weapon_system::{
        DamageFalloff, Weapon, WeaponAttachment, WeaponInventory, WeaponType,
    }; // Weapon system

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::window::{Window, WindowConfig, WindowRunner};

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
