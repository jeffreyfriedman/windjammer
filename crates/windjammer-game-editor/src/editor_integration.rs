// Integration bridge between game-editor panels and windjammer-ui EditorApp
// This module provides a way to add AAA game framework panels to the editor

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use crate::panels::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct GameEditorPanels {
    pub pbr_material_editor: PBRMaterialEditorPanel,
    pub post_processing: PostProcessingPanel,
    pub animation_editor: AnimationEditorPanel,
    pub particle_editor: ParticleEditorPanel,
    pub terrain_editor: TerrainEditorPanel,
    pub ai_behavior_editor: AIBehaviorEditorPanel,
    pub audio_mixer: AudioMixerPanel,
    pub gamepad_config: GamepadConfigPanel,
    pub weapon_editor: WeaponEditorPanel,
    pub navmesh_editor: NavMeshEditorPanel,
    pub profiler: ProfilerPanel,
    
    // Panel visibility state
    pub show_pbr_material: bool,
    pub show_post_processing: bool,
    pub show_animation: bool,
    pub show_particle: bool,
    pub show_terrain: bool,
    pub show_ai_behavior: bool,
    pub show_audio_mixer: bool,
    pub show_gamepad_config: bool,
    pub show_weapon: bool,
    pub show_navmesh: bool,
    pub show_profiler: bool,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for GameEditorPanels {
    fn default() -> Self {
        Self {
            pbr_material_editor: PBRMaterialEditorPanel::new(),
            post_processing: PostProcessingPanel::new(),
            animation_editor: AnimationEditorPanel::new(),
            particle_editor: ParticleEditorPanel::new(),
            terrain_editor: TerrainEditorPanel::new(),
            ai_behavior_editor: AIBehaviorEditorPanel::new(),
            audio_mixer: AudioMixerPanel::new(),
            gamepad_config: GamepadConfigPanel::new(),
            weapon_editor: WeaponEditorPanel::new(),
            navmesh_editor: NavMeshEditorPanel::new(),
            profiler: ProfilerPanel::new(),
            
            show_pbr_material: false,
            show_post_processing: false,
            show_animation: false,
            show_particle: false,
            show_terrain: false,
            show_ai_behavior: false,
            show_audio_mixer: false,
            show_gamepad_config: false,
            show_weapon: false,
            show_navmesh: false,
            show_profiler: false,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl GameEditorPanels {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Render all visible panels as egui windows
    pub fn render(&mut self, ctx: &egui::Context) {
        // PBR Material Editor
        if self.show_pbr_material {
            egui::Window::new("🎨 PBR Material Editor")
                .default_width(400.0)
                .default_height(600.0)
                .vscroll(false)
                .open(&mut self.show_pbr_material)
                .show(ctx, |ui| {
                    self.pbr_material_editor.ui(ui);
                });
        }
        
        // Post-Processing
        if self.show_post_processing {
            egui::Window::new("✨ Post-Processing")
                .default_width(400.0)
                .default_height(600.0)
                .vscroll(false)
                .open(&mut self.show_post_processing)
                .show(ctx, |ui| {
                    self.post_processing.ui(ui);
                });
        }
        
        // Animation Editor
        if self.show_animation {
            egui::Window::new("🎬 Animation State Machine")
                .default_width(800.0)
                .default_height(600.0)
                .open(&mut self.show_animation)
                .show(ctx, |ui| {
                    self.animation_editor.ui(ui);
                });
        }
        
        // Particle Editor
        if self.show_particle {
            egui::Window::new("✨ Particle System Editor")
                .default_width(600.0)
                .default_height(600.0)
                .open(&mut self.show_particle)
                .show(ctx, |ui| {
                    self.particle_editor.ui(ui);
                });
        }
        
        // Terrain Editor
        if self.show_terrain {
            egui::Window::new("🏔️ Terrain Editor")
                .default_width(600.0)
                .default_height(600.0)
                .open(&mut self.show_terrain)
                .show(ctx, |ui| {
                    self.terrain_editor.ui(ui);
                });
        }
        
        // AI Behavior Editor
        if self.show_ai_behavior {
            egui::Window::new("🤖 AI Behavior Tree")
                .default_width(800.0)
                .default_height(600.0)
                .open(&mut self.show_ai_behavior)
                .show(ctx, |ui| {
                    self.ai_behavior_editor.ui(ui);
                });
        }
        
        // Audio Mixer
        if self.show_audio_mixer {
            egui::Window::new("🔊 Audio Mixer")
                .default_width(600.0)
                .default_height(500.0)
                .open(&mut self.show_audio_mixer)
                .show(ctx, |ui| {
                    self.audio_mixer.ui(ui);
                });
        }
        
        // Gamepad Config
        if self.show_gamepad_config {
            egui::Window::new("🎮 Gamepad Configuration")
                .default_width(500.0)
                .default_height(400.0)
                .open(&mut self.show_gamepad_config)
                .show(ctx, |ui| {
                    self.gamepad_config.ui(ui);
                });
        }
        
        // Weapon Editor
        if self.show_weapon {
            egui::Window::new("🔫 Weapon System Editor")
                .default_width(500.0)
                .default_height(500.0)
                .open(&mut self.show_weapon)
                .show(ctx, |ui| {
                    self.weapon_editor.ui(ui);
                });
        }
        
        // NavMesh Editor
        if self.show_navmesh {
            egui::Window::new("🗺️ Navigation Mesh Editor")
                .default_width(600.0)
                .default_height(600.0)
                .open(&mut self.show_navmesh)
                .show(ctx, |ui| {
                    self.navmesh_editor.ui(ui);
                });
        }
        
        // Profiler
        if self.show_profiler {
            egui::Window::new("📊 Performance Profiler")
                .default_width(700.0)
                .default_height(500.0)
                .open(&mut self.show_profiler)
                .show(ctx, |ui| {
                    self.profiler.ui(ui);
                });
        }
    }
    
    /// Render menu items for the View menu
    pub fn render_view_menu(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Game Framework Panels:");
        
        if ui.checkbox(&mut self.show_pbr_material, "🎨 PBR Material Editor").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_post_processing, "✨ Post-Processing").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_animation, "🎬 Animation Editor").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_particle, "✨ Particle Editor").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_terrain, "🏔️ Terrain Editor").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_ai_behavior, "🤖 AI Behavior Tree").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_audio_mixer, "🔊 Audio Mixer").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_gamepad_config, "🎮 Gamepad Config").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_weapon, "🔫 Weapon Editor").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_navmesh, "🗺️ NavMesh Editor").clicked() {
            ui.close_menu();
        }
        
        if ui.checkbox(&mut self.show_profiler, "📊 Profiler").clicked() {
            ui.close_menu();
        }
    }
}

