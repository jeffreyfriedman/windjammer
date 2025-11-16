// Gamepad Configuration Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Ui};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::HashMap;

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum GamepadButton {
    A,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Start,
    Select,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

#[derive(Clone, Debug)]
pub struct ButtonMapping {
    pub button: GamepadButton,
    pub action: String,
}

#[derive(Clone, Debug)]
pub struct AxisMapping {
    pub axis: GamepadAxis,
    pub action: String,
    pub sensitivity: f32,
    pub deadzone: f32,
    pub inverted: bool,
}

pub struct GamepadConfig {
    pub button_mappings: Vec<ButtonMapping>,
    pub axis_mappings: Vec<AxisMapping>,
    pub vibration_enabled: bool,
    pub vibration_strength: f32,
}

impl GamepadConfig {
    pub fn new() -> Self {
        let mut config = Self {
            button_mappings: Vec::new(),
            axis_mappings: Vec::new(),
            vibration_enabled: true,
            vibration_strength: 1.0,
        };

        // Add default mappings
        config.button_mappings.push(ButtonMapping {
            button: GamepadButton::A,
            action: "Jump".to_string(),
        });
        config.button_mappings.push(ButtonMapping {
            button: GamepadButton::B,
            action: "Crouch".to_string(),
        });
        config.button_mappings.push(ButtonMapping {
            button: GamepadButton::X,
            action: "Interact".to_string(),
        });
        config.button_mappings.push(ButtonMapping {
            button: GamepadButton::Y,
            action: "Reload".to_string(),
        });
        config.button_mappings.push(ButtonMapping {
            button: GamepadButton::RightTrigger,
            action: "Fire".to_string(),
        });
        config.button_mappings.push(ButtonMapping {
            button: GamepadButton::LeftTrigger,
            action: "Aim".to_string(),
        });

        config.axis_mappings.push(AxisMapping {
            axis: GamepadAxis::LeftStickX,
            action: "Move Horizontal".to_string(),
            sensitivity: 1.0,
            deadzone: 0.15,
            inverted: false,
        });
        config.axis_mappings.push(AxisMapping {
            axis: GamepadAxis::LeftStickY,
            action: "Move Vertical".to_string(),
            sensitivity: 1.0,
            deadzone: 0.15,
            inverted: false,
        });
        config.axis_mappings.push(AxisMapping {
            axis: GamepadAxis::RightStickX,
            action: "Look Horizontal".to_string(),
            sensitivity: 1.5,
            deadzone: 0.1,
            inverted: false,
        });
        config.axis_mappings.push(AxisMapping {
            axis: GamepadAxis::RightStickY,
            action: "Look Vertical".to_string(),
            sensitivity: 1.5,
            deadzone: 0.1,
            inverted: true,
        });

        config
    }
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct GamepadConfigPanel {
    config: GamepadConfig,
    selected_button_mapping: Option<usize>,
    selected_axis_mapping: Option<usize>,
    new_action_name: String,
    test_mode: bool,
    test_values: HashMap<String, f32>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl GamepadConfigPanel {
    pub fn new() -> Self {
        Self {
            config: GamepadConfig::new(),
            selected_button_mapping: None,
            selected_axis_mapping: None,
            new_action_name: String::new(),
            test_mode: false,
            test_values: HashMap::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🎮 Gamepad Configuration");
            ui.separator();
            if ui.button(if self.test_mode { "⏸️ Stop Test" } else { "▶️ Test" }).clicked() {
                self.test_mode = !self.test_mode;
            }
            if ui.button("💾 Save Config").clicked() {
                // TODO: Save configuration to file
            }
            if ui.button("📂 Load Config").clicked() {
                // TODO: Load configuration from file
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Left: Mappings
            ui.vertical(|ui| {
                ui.set_min_width(400.0);
                self.render_mappings(ui);
            });

            ui.separator();

            // Right: Properties and test
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                if self.test_mode {
                    self.render_test_view(ui);
                } else {
                    self.render_properties(ui);
                }
            });
        });
    }

    fn render_mappings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing("Button Mappings", |ui| {
                let mut to_remove = None;
                for (idx, mapping) in self.config.button_mappings.iter_mut().enumerate() {
                    let is_selected = Some(idx) == self.selected_button_mapping;
                    ui.horizontal(|ui| {
                        if ui.selectable_label(is_selected, format!("{:?}", mapping.button)).clicked() {
                            self.selected_button_mapping = Some(idx);
                            self.selected_axis_mapping = None;
                        }
                        ui.label("→");
                        ui.text_edit_singleline(&mut mapping.action);
                        if ui.button("🗑️").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                }

                if let Some(idx) = to_remove {
                    self.config.button_mappings.remove(idx);
                    if self.selected_button_mapping == Some(idx) {
                        self.selected_button_mapping = None;
                    }
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Add mapping:");
                    ui.text_edit_singleline(&mut self.new_action_name);
                    if ui.button("➕ Add").clicked() && !self.new_action_name.is_empty() {
                        self.config.button_mappings.push(ButtonMapping {
                            button: GamepadButton::A,
                            action: self.new_action_name.clone(),
                        });
                        self.new_action_name.clear();
                    }
                });
            });

            ui.separator();

            ui.collapsing("Axis Mappings", |ui| {
                let mut to_remove = None;
                for (idx, mapping) in self.config.axis_mappings.iter_mut().enumerate() {
                    let is_selected = Some(idx) == self.selected_axis_mapping;
                    ui.horizontal(|ui| {
                        if ui.selectable_label(is_selected, format!("{:?}", mapping.axis)).clicked() {
                            self.selected_axis_mapping = Some(idx);
                            self.selected_button_mapping = None;
                        }
                        ui.label("→");
                        ui.label(&mapping.action);
                        if ui.button("🗑️").clicked() {
                            to_remove = Some(idx);
                        }
                    });

                    if is_selected {
                        ui.indent(idx, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Sensitivity:");
                                ui.add(egui::Slider::new(&mut mapping.sensitivity, 0.1..=5.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Deadzone:");
                                ui.add(egui::Slider::new(&mut mapping.deadzone, 0.0..=0.5));
                            });
                            ui.checkbox(&mut mapping.inverted, "Inverted");
                        });
                    }
                }

                if let Some(idx) = to_remove {
                    self.config.axis_mappings.remove(idx);
                    if self.selected_axis_mapping == Some(idx) {
                        self.selected_axis_mapping = None;
                    }
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Add axis mapping:");
                    ui.text_edit_singleline(&mut self.new_action_name);
                    if ui.button("➕ Add").clicked() && !self.new_action_name.is_empty() {
                        self.config.axis_mappings.push(AxisMapping {
                            axis: GamepadAxis::LeftStickX,
                            action: self.new_action_name.clone(),
                            sensitivity: 1.0,
                            deadzone: 0.15,
                            inverted: false,
                        });
                        self.new_action_name.clear();
                    }
                });
            });
        });
    }

    fn render_properties(&mut self, ui: &mut Ui) {
        ui.heading("Properties");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing("Vibration Settings", |ui| {
                ui.checkbox(&mut self.config.vibration_enabled, "Enable Vibration");
                if self.config.vibration_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        ui.add(egui::Slider::new(&mut self.config.vibration_strength, 0.0..=1.0));
                    });
                }
            });

            ui.separator();

            if let Some(idx) = self.selected_button_mapping {
                if let Some(mapping) = self.config.button_mappings.get_mut(idx) {
                    ui.label("📍 Selected Button Mapping");
                    ui.separator();

                    ui.label(format!("Button: {:?}", mapping.button));
                    ui.horizontal(|ui| {
                        ui.label("Action:");
                        ui.text_edit_singleline(&mut mapping.action);
                    });

                    ui.separator();

                    ui.label("Change Button:");
                    ui.horizontal_wrapped(|ui| {
                        let buttons = [
                            GamepadButton::A,
                            GamepadButton::B,
                            GamepadButton::X,
                            GamepadButton::Y,
                            GamepadButton::LeftBumper,
                            GamepadButton::RightBumper,
                            GamepadButton::LeftTrigger,
                            GamepadButton::RightTrigger,
                            GamepadButton::Start,
                            GamepadButton::Select,
                            GamepadButton::LeftStick,
                            GamepadButton::RightStick,
                            GamepadButton::DPadUp,
                            GamepadButton::DPadDown,
                            GamepadButton::DPadLeft,
                            GamepadButton::DPadRight,
                        ];

                        for button in buttons {
                            if ui.selectable_label(mapping.button == button, format!("{:?}", button)).clicked() {
                                mapping.button = button;
                            }
                        }
                    });
                }
            } else if let Some(idx) = self.selected_axis_mapping {
                if let Some(mapping) = self.config.axis_mappings.get_mut(idx) {
                    ui.label("📍 Selected Axis Mapping");
                    ui.separator();

                    ui.label(format!("Axis: {:?}", mapping.axis));
                    ui.horizontal(|ui| {
                        ui.label("Action:");
                        ui.text_edit_singleline(&mut mapping.action);
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        ui.add(egui::Slider::new(&mut mapping.sensitivity, 0.1..=5.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Deadzone:");
                        ui.add(egui::Slider::new(&mut mapping.deadzone, 0.0..=0.5));
                    });

                    ui.checkbox(&mut mapping.inverted, "Inverted");

                    ui.separator();

                    ui.label("Change Axis:");
                    let axes = [
                        GamepadAxis::LeftStickX,
                        GamepadAxis::LeftStickY,
                        GamepadAxis::RightStickX,
                        GamepadAxis::RightStickY,
                        GamepadAxis::LeftTrigger,
                        GamepadAxis::RightTrigger,
                    ];

                    for axis in axes {
                        if ui.selectable_label(mapping.axis == axis, format!("{:?}", axis)).clicked() {
                            mapping.axis = axis;
                        }
                    }
                }
            } else {
                ui.label("No mapping selected");
                ui.separator();
                ui.label("Select a button or axis mapping to edit");
            }
        });
    }

    fn render_test_view(&mut self, ui: &mut Ui) {
        ui.heading("Test Mode");
        ui.separator();

        ui.label("🎮 Connect a gamepad and test your mappings");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Button States:");
            for mapping in &self.config.button_mappings {
                ui.horizontal(|ui| {
                    let pressed = self.test_values.get(&mapping.action).copied().unwrap_or(0.0) > 0.5;
                    let color = if pressed { Color32::GREEN } else { Color32::GRAY };
                    ui.colored_label(color, format!("{:?}", mapping.button));
                    ui.label("→");
                    ui.label(&mapping.action);
                });
            }

            ui.separator();

            ui.label("Axis Values:");
            for mapping in &self.config.axis_mappings {
                let value = self.test_values.get(&mapping.action).copied().unwrap_or(0.0);
                ui.horizontal(|ui| {
                    ui.label(format!("{:?}", mapping.axis));
                    ui.label("→");
                    ui.label(&mapping.action);
                });
                ui.add(egui::ProgressBar::new((value + 1.0) / 2.0).show_percentage());
            }

            ui.separator();

            ui.label("💡 Tip: Press buttons and move sticks to see values update");
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for GamepadConfigPanel {
    fn default() -> Self {
        Self::new()
    }
}
