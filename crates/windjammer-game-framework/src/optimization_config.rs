//! # Optimization Configuration System
//!
//! Unified configuration system for all automatic optimization features.
//!
//! ## Features
//! - Centralized optimization settings
//! - Runtime configuration changes
//! - Preset optimization profiles
//! - Per-feature enable/disable flags
//! - Performance vs quality trade-offs
//! - Configuration serialization (JSON/TOML)
//! - Hot-reload configuration
//! - Platform-specific defaults
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::optimization_config::{OptimizationConfig, OptimizationProfile};
//!
//! let mut config = OptimizationConfig::from_profile(OptimizationProfile::Balanced);
//! config.batching.enabled = true;
//! config.culling.enabled = true;
//! config.apply();
//! ```

use serde::{Deserialize, Serialize};

/// Optimization profile presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationProfile {
    /// Maximum quality, minimal optimizations
    Quality,
    /// Balanced quality and performance
    Balanced,
    /// Maximum performance, aggressive optimizations
    Performance,
    /// Custom configuration
    Custom,
}

/// Batching optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptBatchingConfig {
    /// Enable draw call batching
    pub enabled: bool,
    /// Enable instanced rendering
    pub enable_instancing: bool,
    /// Enable dynamic batching
    pub enable_dynamic_batching: bool,
    /// Enable static batching
    pub enable_static_batching: bool,
    /// Maximum vertices per batch
    pub max_vertices_per_batch: usize,
    /// Maximum instances per batch
    pub max_instances_per_batch: usize,
    /// Minimum instances for instancing
    pub min_instances_for_instancing: usize,
}

impl Default for OptBatchingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_instancing: true,
            enable_dynamic_batching: true,
            enable_static_batching: true,
            max_vertices_per_batch: 10000,
            max_instances_per_batch: 1000,
            min_instances_for_instancing: 2,
        }
    }
}

/// Culling optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptCullingConfig {
    /// Enable culling
    pub enabled: bool,
    /// Enable frustum culling
    pub enable_frustum_culling: bool,
    /// Enable distance culling
    pub enable_distance_culling: bool,
    /// Maximum render distance
    pub max_render_distance: f32,
    /// Enable layer-based culling
    pub enable_layer_culling: bool,
    /// Enable occlusion culling
    pub enable_occlusion_culling: bool,
}

impl Default for OptCullingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_frustum_culling: true,
            enable_distance_culling: true,
            max_render_distance: 1000.0,
            enable_layer_culling: false,
            enable_occlusion_culling: false,
        }
    }
}

/// LOD (Level of Detail) optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptLODConfig {
    /// Enable LOD system
    pub enabled: bool,
    /// LOD bias (multiplier for distances)
    pub lod_bias: f32,
    /// Enable smooth transitions
    pub enable_smooth_transitions: bool,
    /// Transition duration (seconds)
    pub transition_duration: f32,
    /// Use screen coverage for LOD selection
    pub use_screen_coverage: bool,
    /// Maximum LOD level
    pub max_lod_level: usize,
}

impl Default for OptLODConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lod_bias: 1.0,
            enable_smooth_transitions: true,
            transition_duration: 0.5,
            use_screen_coverage: false,
            max_lod_level: 4,
        }
    }
}

/// Memory pooling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolingConfig {
    /// Enable memory pooling
    pub enabled: bool,
    /// Initial pool capacity
    pub initial_capacity: usize,
    /// Maximum pool size (0 = unlimited)
    pub max_capacity: usize,
    /// Enable automatic growth
    pub auto_grow: bool,
    /// Growth factor
    pub growth_factor: f32,
    /// Enable automatic shrinking
    pub auto_shrink: bool,
}

impl Default for MemoryPoolingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_capacity: 32,
            max_capacity: 0,
            auto_grow: true,
            growth_factor: 2.0,
            auto_shrink: false,
        }
    }
}

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable profiling
    pub enabled: bool,
    /// Maximum number of frames to keep in history
    pub max_history_frames: usize,
    /// Enable statistical analysis
    pub enable_statistics: bool,
    /// Enable memory tracking
    pub enable_memory_tracking: bool,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_history_frames: 300,
            enable_statistics: true,
            enable_memory_tracking: false,
        }
    }
}

/// Rendering optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    /// Enable GPU instancing
    pub enable_gpu_instancing: bool,
    /// Enable texture atlasing
    pub enable_texture_atlasing: bool,
    /// Enable shader caching
    pub enable_shader_caching: bool,
    /// Enable render state caching
    pub enable_state_caching: bool,
    /// Maximum draw calls per frame
    pub max_draw_calls_per_frame: usize,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            enable_gpu_instancing: true,
            enable_texture_atlasing: true,
            enable_shader_caching: true,
            enable_state_caching: true,
            max_draw_calls_per_frame: 5000,
        }
    }
}

/// Physics optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Enable physics optimizations
    pub enabled: bool,
    /// Enable spatial partitioning
    pub enable_spatial_partitioning: bool,
    /// Enable sleeping bodies
    pub enable_sleeping: bool,
    /// Maximum physics substeps
    pub max_substeps: usize,
    /// Physics update rate (Hz)
    pub update_rate: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_spatial_partitioning: true,
            enable_sleeping: true,
            max_substeps: 4,
            update_rate: 60.0,
        }
    }
}

/// Audio optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Enable audio optimizations
    pub enabled: bool,
    /// Maximum active sounds
    pub max_active_sounds: usize,
    /// Enable audio streaming
    pub enable_streaming: bool,
    /// Enable audio compression
    pub enable_compression: bool,
    /// Audio quality (0.0 to 1.0)
    pub quality: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_active_sounds: 32,
            enable_streaming: true,
            enable_compression: true,
            quality: 0.8,
        }
    }
}

/// Main optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Current profile
    pub profile: OptimizationProfile,
    /// Batching configuration
    pub batching: OptBatchingConfig,
    /// Culling configuration
    pub culling: OptCullingConfig,
    /// LOD configuration
    pub lod: OptLODConfig,
    /// Memory pooling configuration
    pub memory_pooling: MemoryPoolingConfig,
    /// Profiling configuration
    pub profiling: ProfilingConfig,
    /// Rendering configuration
    pub rendering: RenderingConfig,
    /// Physics configuration
    pub physics: PhysicsConfig,
    /// Audio configuration
    pub audio: AudioConfig,
}

impl OptimizationConfig {
    /// Create a new configuration from a profile
    pub fn from_profile(profile: OptimizationProfile) -> Self {
        let mut config = Self::default();
        config.profile = profile;

        match profile {
            OptimizationProfile::Quality => config.apply_quality_preset(),
            OptimizationProfile::Balanced => config.apply_balanced_preset(),
            OptimizationProfile::Performance => config.apply_performance_preset(),
            OptimizationProfile::Custom => {}
        }

        config
    }

    /// Apply quality preset
    fn apply_quality_preset(&mut self) {
        // Minimal optimizations, maximum quality
        self.batching.enabled = false;
        self.culling.max_render_distance = 5000.0;
        self.lod.enabled = false;
        self.rendering.max_draw_calls_per_frame = 10000;
        self.physics.max_substeps = 8;
        self.audio.quality = 1.0;
        self.audio.enable_compression = false;
    }

    /// Apply balanced preset
    fn apply_balanced_preset(&mut self) {
        // Balanced settings (defaults are already balanced)
    }

    /// Apply performance preset
    fn apply_performance_preset(&mut self) {
        // Aggressive optimizations
        self.batching.enabled = true;
        self.batching.enable_instancing = true;
        self.batching.enable_dynamic_batching = true;
        self.batching.enable_static_batching = true;
        
        self.culling.enabled = true;
        self.culling.enable_frustum_culling = true;
        self.culling.enable_distance_culling = true;
        self.culling.enable_occlusion_culling = true;
        self.culling.max_render_distance = 500.0;
        
        self.lod.enabled = true;
        self.lod.lod_bias = 0.8;
        self.lod.use_screen_coverage = true;
        
        self.memory_pooling.enabled = true;
        self.memory_pooling.auto_grow = true;
        self.memory_pooling.auto_shrink = true;
        
        self.rendering.enable_gpu_instancing = true;
        self.rendering.enable_texture_atlasing = true;
        self.rendering.max_draw_calls_per_frame = 2000;
        
        self.physics.max_substeps = 2;
        self.physics.enable_sleeping = true;
        
        self.audio.max_active_sounds = 16;
        self.audio.quality = 0.6;
        self.audio.enable_compression = true;
    }

    /// Apply the configuration
    pub fn apply(&self) {
        // This would be called to apply the configuration to the engine
        // In a real implementation, this would update all the relevant systems
    }

    /// Save configuration to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load configuration from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Save configuration to TOML
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Load configuration from TOML
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Get platform-specific defaults
    pub fn platform_defaults() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::from_profile(OptimizationProfile::Balanced)
        }

        #[cfg(target_os = "macos")]
        {
            Self::from_profile(OptimizationProfile::Balanced)
        }

        #[cfg(target_os = "linux")]
        {
            Self::from_profile(OptimizationProfile::Balanced)
        }

        #[cfg(target_arch = "wasm32")]
        {
            Self::from_profile(OptimizationProfile::Performance)
        }

        #[cfg(target_os = "android")]
        {
            Self::from_profile(OptimizationProfile::Performance)
        }

        #[cfg(target_os = "ios")]
        {
            Self::from_profile(OptimizationProfile::Performance)
        }

        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_arch = "wasm32",
            target_os = "android",
            target_os = "ios"
        )))]
        {
            Self::default()
        }
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            profile: OptimizationProfile::Balanced,
            batching: OptBatchingConfig::default(),
            culling: OptCullingConfig::default(),
            lod: OptLODConfig::default(),
            memory_pooling: MemoryPoolingConfig::default(),
            profiling: ProfilingConfig::default(),
            rendering: RenderingConfig::default(),
            physics: PhysicsConfig::default(),
            audio: AudioConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_profile() {
        let profile = OptimizationProfile::Balanced;
        assert_eq!(profile, OptimizationProfile::Balanced);
    }

    #[test]
    fn test_batching_config_default() {
        let config = OptBatchingConfig::default();
        assert!(config.enabled);
        assert!(config.enable_instancing);
    }

    #[test]
    fn test_culling_config_default() {
        let config = OptCullingConfig::default();
        assert!(config.enabled);
        assert!(config.enable_frustum_culling);
        assert_eq!(config.max_render_distance, 1000.0);
    }

    #[test]
    fn test_lod_config_default() {
        let config = OptLODConfig::default();
        assert!(config.enabled);
        assert_eq!(config.lod_bias, 1.0);
        assert!(config.enable_smooth_transitions);
    }

    #[test]
    fn test_memory_pooling_config_default() {
        let config = MemoryPoolingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.initial_capacity, 32);
    }

    #[test]
    fn test_profiling_config_default() {
        let config = ProfilingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_history_frames, 300);
    }

    #[test]
    fn test_rendering_config_default() {
        let config = RenderingConfig::default();
        assert!(config.enable_gpu_instancing);
        assert!(config.enable_shader_caching);
    }

    #[test]
    fn test_physics_config_default() {
        let config = PhysicsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.update_rate, 60.0);
    }

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_active_sounds, 32);
    }

    #[test]
    fn test_optimization_config_default() {
        let config = OptimizationConfig::default();
        assert_eq!(config.profile, OptimizationProfile::Balanced);
        assert!(config.batching.enabled);
    }

    #[test]
    fn test_quality_profile() {
        let config = OptimizationConfig::from_profile(OptimizationProfile::Quality);
        assert_eq!(config.profile, OptimizationProfile::Quality);
        assert!(!config.batching.enabled);
        assert!(!config.lod.enabled);
    }

    #[test]
    fn test_balanced_profile() {
        let config = OptimizationConfig::from_profile(OptimizationProfile::Balanced);
        assert_eq!(config.profile, OptimizationProfile::Balanced);
    }

    #[test]
    fn test_performance_profile() {
        let config = OptimizationConfig::from_profile(OptimizationProfile::Performance);
        assert_eq!(config.profile, OptimizationProfile::Performance);
        assert!(config.batching.enabled);
        assert!(config.culling.enabled);
        assert!(config.lod.enabled);
        assert_eq!(config.culling.max_render_distance, 500.0);
    }

    #[test]
    fn test_custom_profile() {
        let config = OptimizationConfig::from_profile(OptimizationProfile::Custom);
        assert_eq!(config.profile, OptimizationProfile::Custom);
    }

    #[test]
    fn test_json_serialization() {
        let config = OptimizationConfig::default();
        let json = config.to_json().unwrap();
        assert!(json.contains("profile"));
        assert!(json.contains("batching"));
    }

    #[test]
    fn test_json_deserialization() {
        let config = OptimizationConfig::default();
        let json = config.to_json().unwrap();
        let loaded = OptimizationConfig::from_json(&json).unwrap();
        assert_eq!(loaded.profile, config.profile);
    }

    #[test]
    fn test_toml_serialization() {
        let config = OptimizationConfig::default();
        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("profile"));
        assert!(toml_str.contains("batching"));
    }

    #[test]
    fn test_toml_deserialization() {
        let config = OptimizationConfig::default();
        let toml_str = config.to_toml().unwrap();
        let loaded = OptimizationConfig::from_toml(&toml_str).unwrap();
        assert_eq!(loaded.profile, config.profile);
    }

    #[test]
    fn test_platform_defaults() {
        let config = OptimizationConfig::platform_defaults();
        assert!(matches!(
            config.profile,
            OptimizationProfile::Balanced | OptimizationProfile::Performance
        ));
    }

    #[test]
    fn test_apply_configuration() {
        let config = OptimizationConfig::default();
        config.apply(); // Should not panic
    }

    #[test]
    fn test_batching_config_modification() {
        let mut config = OptBatchingConfig::default();
        config.max_vertices_per_batch = 5000;
        assert_eq!(config.max_vertices_per_batch, 5000);
    }

    #[test]
    fn test_culling_config_modification() {
        let mut config = OptCullingConfig::default();
        config.max_render_distance = 2000.0;
        assert_eq!(config.max_render_distance, 2000.0);
    }

    #[test]
    fn test_lod_config_modification() {
        let mut config = OptLODConfig::default();
        config.lod_bias = 1.5;
        assert_eq!(config.lod_bias, 1.5);
    }

    #[test]
    fn test_memory_pooling_config_modification() {
        let mut config = MemoryPoolingConfig::default();
        config.initial_capacity = 64;
        assert_eq!(config.initial_capacity, 64);
    }

    #[test]
    fn test_profiling_config_modification() {
        let mut config = ProfilingConfig::default();
        config.max_history_frames = 600;
        assert_eq!(config.max_history_frames, 600);
    }

    #[test]
    fn test_rendering_config_modification() {
        let mut config = RenderingConfig::default();
        config.max_draw_calls_per_frame = 3000;
        assert_eq!(config.max_draw_calls_per_frame, 3000);
    }

    #[test]
    fn test_physics_config_modification() {
        let mut config = PhysicsConfig::default();
        config.update_rate = 120.0;
        assert_eq!(config.update_rate, 120.0);
    }

    #[test]
    fn test_audio_config_modification() {
        let mut config = AudioConfig::default();
        config.quality = 0.9;
        assert_eq!(config.quality, 0.9);
    }

    #[test]
    fn test_profile_switching() {
        let mut config = OptimizationConfig::from_profile(OptimizationProfile::Quality);
        assert!(!config.batching.enabled);

        config = OptimizationConfig::from_profile(OptimizationProfile::Performance);
        assert!(config.batching.enabled);
    }

    #[test]
    fn test_aggressive_performance_settings() {
        let config = OptimizationConfig::from_profile(OptimizationProfile::Performance);
        assert!(config.culling.enable_occlusion_culling);
        assert!(config.lod.use_screen_coverage);
        assert!(config.memory_pooling.auto_shrink);
    }

    #[test]
    fn test_quality_vs_performance_tradeoff() {
        let quality = OptimizationConfig::from_profile(OptimizationProfile::Quality);
        let performance = OptimizationConfig::from_profile(OptimizationProfile::Performance);

        assert!(quality.culling.max_render_distance > performance.culling.max_render_distance);
        assert!(quality.audio.quality > performance.audio.quality);
        assert!(quality.physics.max_substeps > performance.physics.max_substeps);
    }
}

