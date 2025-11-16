// Editor Panels for AAA Game Framework Integration

pub mod pbr_material_editor;
pub mod post_processing_editor;
pub mod animation_editor;
pub mod particle_editor;
pub mod terrain_editor;
pub mod ai_behavior_editor;
pub mod audio_mixer;
pub mod gamepad_config;
pub mod weapon_editor;
pub mod navmesh_editor;
pub mod profiler_panel;

// Re-exports
pub use pbr_material_editor::PBRMaterialEditorPanel;
pub use post_processing_editor::PostProcessingPanel;
pub use animation_editor::AnimationEditor;
pub use particle_editor::ParticleEditorPanel;
pub use terrain_editor::TerrainEditor;
pub use ai_behavior_editor::AIBehaviorEditor;
pub use audio_mixer::AudioMixer;
pub use gamepad_config::GamepadConfigPanel;
pub use weapon_editor::WeaponEditor;
pub use navmesh_editor::NavMeshEditor;
pub use profiler_panel::ProfilerPanel;

