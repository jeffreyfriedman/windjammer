// Audio Mixer Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Ui};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::HashMap;

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug)]
pub struct AudioBus {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
    pub parent: Option<String>,
    pub effects: Vec<AudioEffect>,
}

#[derive(Clone, Debug)]
pub struct AudioEffect {
    pub effect_type: EffectType,
    pub enabled: bool,
    pub parameters: HashMap<String, f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectType {
    Reverb,
    Delay,
    Chorus,
    Distortion,
    EQ,
    Compressor,
    LowPass,
    HighPass,
}

impl EffectType {
    fn default_parameters(&self) -> HashMap<String, f32> {
        let mut params = HashMap::new();
        match self {
            EffectType::Reverb => {
                params.insert("room_size".to_string(), 0.5);
                params.insert("damping".to_string(), 0.5);
                params.insert("wet".to_string(), 0.3);
            }
            EffectType::Delay => {
                params.insert("time".to_string(), 0.5);
                params.insert("feedback".to_string(), 0.3);
                params.insert("wet".to_string(), 0.5);
            }
            EffectType::Chorus => {
                params.insert("rate".to_string(), 1.0);
                params.insert("depth".to_string(), 0.5);
                params.insert("mix".to_string(), 0.5);
            }
            EffectType::Distortion => {
                params.insert("drive".to_string(), 0.5);
                params.insert("tone".to_string(), 0.5);
                params.insert("mix".to_string(), 1.0);
            }
            EffectType::EQ => {
                params.insert("low".to_string(), 0.0);
                params.insert("mid".to_string(), 0.0);
                params.insert("high".to_string(), 0.0);
            }
            EffectType::Compressor => {
                params.insert("threshold".to_string(), -10.0);
                params.insert("ratio".to_string(), 4.0);
                params.insert("attack".to_string(), 5.0);
                params.insert("release".to_string(), 50.0);
            }
            EffectType::LowPass => {
                params.insert("cutoff".to_string(), 1000.0);
                params.insert("resonance".to_string(), 0.5);
            }
            EffectType::HighPass => {
                params.insert("cutoff".to_string(), 100.0);
                params.insert("resonance".to_string(), 0.5);
            }
        }
        params
    }
}

pub struct AudioMixerData {
    pub buses: HashMap<String, AudioBus>,
    pub master_volume: f32,
}

impl AudioMixerData {
    pub fn new() -> Self {
        let mut buses = HashMap::new();
        
        // Create default buses
        buses.insert(
            "Master".to_string(),
            AudioBus {
                name: "Master".to_string(),
                volume: 1.0,
                muted: false,
                solo: false,
                parent: None,
                effects: Vec::new(),
            },
        );
        
        buses.insert(
            "Music".to_string(),
            AudioBus {
                name: "Music".to_string(),
                volume: 0.7,
                muted: false,
                solo: false,
                parent: Some("Master".to_string()),
                effects: Vec::new(),
            },
        );
        
        buses.insert(
            "SFX".to_string(),
            AudioBus {
                name: "SFX".to_string(),
                volume: 0.8,
                muted: false,
                solo: false,
                parent: Some("Master".to_string()),
                effects: Vec::new(),
            },
        );
        
        buses.insert(
            "Voice".to_string(),
            AudioBus {
                name: "Voice".to_string(),
                volume: 0.9,
                muted: false,
                solo: false,
                parent: Some("Master".to_string()),
                effects: Vec::new(),
            },
        );

        Self {
            buses,
            master_volume: 1.0,
        }
    }

    pub fn add_bus(&mut self, name: String, parent: Option<String>) {
        self.buses.insert(
            name.clone(),
            AudioBus {
                name,
                volume: 1.0,
                muted: false,
                solo: false,
                parent,
                effects: Vec::new(),
            },
        );
    }

    pub fn remove_bus(&mut self, name: &str) {
        if name != "Master" {
            self.buses.remove(name);
            // Remove this bus as parent from other buses
            for bus in self.buses.values_mut() {
                if bus.parent.as_deref() == Some(name) {
                    bus.parent = Some("Master".to_string());
                }
            }
        }
    }
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AudioMixer {
    mixer_data: AudioMixerData,
    selected_bus: Option<String>,
    new_bus_name: String,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AudioMixer {
    pub fn new() -> Self {
        Self {
            mixer_data: AudioMixerData::new(),
            selected_bus: Some("Master".to_string()),
            new_bus_name: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🔊 Audio Mixer");
            ui.separator();
            if ui.button("➕ Add Bus").clicked() {
                if !self.new_bus_name.is_empty() {
                    self.mixer_data.add_bus(self.new_bus_name.clone(), Some("Master".to_string()));
                    self.new_bus_name.clear();
                }
            }
            ui.text_edit_singleline(&mut self.new_bus_name);
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Left: Bus list and mixer view
            ui.vertical(|ui| {
                ui.set_min_width(500.0);
                self.render_mixer_view(ui);
            });

            ui.separator();

            // Right: Bus properties and effects
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                self.render_bus_properties(ui);
            });
        });
    }

    fn render_mixer_view(&mut self, ui: &mut Ui) {
        ui.label("Master Volume");
        ui.add(egui::Slider::new(&mut self.mixer_data.master_volume, 0.0..=1.0).vertical());

        ui.separator();

        ui.label("Audio Buses");
        
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                let bus_names: Vec<_> = self.mixer_data.buses.keys().cloned().collect();

                for bus_name in bus_names {
                    if let Some(bus) = self.mixer_data.buses.get(&bus_name) {
                        // Clone the bus data for rendering
                        let bus_clone = bus.clone();
                        self.render_bus_channel(ui, &bus_clone, &bus_name);
                    }
                }
            });
        });
    }

    fn render_bus_channel(&mut self, ui: &mut Ui, bus: &AudioBus, bus_name: &str) {
        ui.vertical(|ui| {
            ui.set_width(100.0);
            
            let is_selected = self.selected_bus.as_deref() == Some(&bus.name);
            let bg_color = if is_selected {
                Color32::from_rgb(60, 80, 120)
            } else {
                Color32::from_gray(40)
            };

            egui::Frame::none()
                .fill(bg_color)
                .inner_margin(5.0)
                .show(ui, |ui| {
                    // Bus name (clickable)
                    if ui.selectable_label(is_selected, &bus.name).clicked() {
                        self.selected_bus = Some(bus.name.clone());
                    }

                    ui.separator();

                    // Volume fader (vertical slider)
                    let mut volume = bus.volume;
                    ui.add(
                        egui::Slider::new(&mut volume, 0.0..=1.0)
                            .vertical()
                            .show_value(false),
                    );
                    if volume != bus.volume {
                        if let Some(bus_mut) = self.mixer_data.buses.get_mut(bus_name) {
                            bus_mut.volume = volume;
                        }
                    }

                    // Volume label
                    ui.label(format!("{:.0}%", volume * 100.0));

                    ui.separator();

                    // Mute/Solo buttons
                    ui.horizontal(|ui| {
                        let mut muted = bus.muted;
                        let mut solo = bus.solo;

                        if ui.selectable_label(muted, "M").clicked() {
                            muted = !muted;
                            if let Some(bus_mut) = self.mixer_data.buses.get_mut(bus_name) {
                                bus_mut.muted = muted;
                            }
                        }

                        if ui.selectable_label(solo, "S").clicked() {
                            solo = !solo;
                            if let Some(bus_mut) = self.mixer_data.buses.get_mut(bus_name) {
                                bus_mut.solo = solo;
                            }
                        }
                    });

                    // Effects indicator
                    if !bus.effects.is_empty() {
                        ui.label(format!("🎛️ {}", bus.effects.len()));
                    }
                });
        });
    }

    fn render_bus_properties(&mut self, ui: &mut Ui) {
        ui.heading("Bus Properties");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(bus_name) = &self.selected_bus.clone() {
                // Collect available buses before mutable borrow
                let available_buses: Vec<_> = self.mixer_data.buses.keys().cloned().collect();
                
                if let Some(bus) = self.mixer_data.buses.get_mut(bus_name) {
                    ui.label(format!("Bus: {}", bus.name));
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Volume:");
                        ui.add(egui::Slider::new(&mut bus.volume, 0.0..=1.0));
                    });

                    ui.checkbox(&mut bus.muted, "Muted");
                    ui.checkbox(&mut bus.solo, "Solo");

                    ui.separator();

                    // Parent bus selection
                    ui.label("Parent Bus:");
                    let current_parent = bus.parent.clone().unwrap_or_else(|| "None".to_string());
                    let bus_name_for_comparison = bus.name.clone();
                    let current_parent_for_comparison = bus.parent.clone();
                    
                    egui::ComboBox::from_label("Parent")
                        .selected_text(&current_parent)
                        .show_ui(ui, |ui| {
                            for parent_name in available_buses {
                                if parent_name != bus_name_for_comparison {
                                    if ui.selectable_label(current_parent_for_comparison.as_deref() == Some(&parent_name), &parent_name).clicked() {
                                        bus.parent = Some(parent_name);
                                    }
                                }
                            }
                        });

                    ui.separator();

                    // Effects chain
                    ui.collapsing("Effects Chain", |ui| {
                        let mut to_remove = None;
                        for (idx, effect) in bus.effects.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut effect.enabled, "");
                                ui.label(format!("{:?}", effect.effect_type));
                                if ui.button("🗑️").clicked() {
                                    to_remove = Some(idx);
                                }
                            });

                            if effect.enabled {
                                ui.indent(idx, |ui| {
                                    for (param_name, param_value) in effect.parameters.iter_mut() {
                                        ui.horizontal(|ui| {
                                            ui.label(param_name);
                                            ui.add(egui::Slider::new(param_value, -1.0..=1.0));
                                        });
                                    }
                                });
                            }
                        }

                        if let Some(idx) = to_remove {
                            bus.effects.remove(idx);
                        }

                        ui.separator();

                        ui.menu_button("➕ Add Effect", |ui| {
                            if ui.button("Reverb").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::Reverb,
                                    enabled: true,
                                    parameters: EffectType::Reverb.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("Delay").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::Delay,
                                    enabled: true,
                                    parameters: EffectType::Delay.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("Chorus").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::Chorus,
                                    enabled: true,
                                    parameters: EffectType::Chorus.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("Distortion").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::Distortion,
                                    enabled: true,
                                    parameters: EffectType::Distortion.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("EQ").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::EQ,
                                    enabled: true,
                                    parameters: EffectType::EQ.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("Compressor").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::Compressor,
                                    enabled: true,
                                    parameters: EffectType::Compressor.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("Low Pass").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::LowPass,
                                    enabled: true,
                                    parameters: EffectType::LowPass.default_parameters(),
                                });
                                ui.close_menu();
                            }
                            if ui.button("High Pass").clicked() {
                                bus.effects.push(AudioEffect {
                                    effect_type: EffectType::HighPass,
                                    enabled: true,
                                    parameters: EffectType::HighPass.default_parameters(),
                                });
                                ui.close_menu();
                            }
                        });
                    });

                    ui.separator();

                    if bus.name != "Master" && ui.button("🗑️ Delete Bus").clicked() {
                        let name_to_remove = bus.name.clone();
                        self.selected_bus = Some("Master".to_string());
                        self.mixer_data.remove_bus(&name_to_remove);
                    }
                }
            } else {
                ui.label("No bus selected");
            }
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}
