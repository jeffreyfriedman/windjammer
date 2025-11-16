// Weapon System Editor Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::path::PathBuf;

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum WeaponType {
    Pistol,
    Rifle,
    Shotgun,
    Sniper,
    SMG,
    Melee,
    Explosive,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FireMode {
    SemiAuto,
    FullAuto,
    Burst(u32), // Burst size
}

#[derive(Clone, Debug)]
pub struct WeaponStats {
    pub weapon_type: WeaponType,
    pub fire_mode: FireMode,
    pub damage: f32,
    pub fire_rate: f32, // Rounds per minute
    pub magazine_size: u32,
    pub reload_time: f32,
    pub range: f32,
    pub accuracy: f32,
    pub recoil: f32,
    pub damage_falloff_start: f32,
    pub damage_falloff_end: f32,
}

#[derive(Clone, Debug)]
pub struct WeaponAttachment {
    pub name: String,
    pub attachment_type: AttachmentType,
    pub stat_modifiers: Vec<StatModifier>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttachmentType {
    Scope,
    Barrel,
    Grip,
    Magazine,
    Stock,
}

#[derive(Clone, Debug)]
pub struct StatModifier {
    pub stat_name: String,
    pub modifier: f32,
    pub is_multiplier: bool,
}

pub struct Weapon {
    pub name: String,
    pub stats: WeaponStats,
    pub model_path: PathBuf,
    pub icon_path: PathBuf,
    pub attachments: Vec<WeaponAttachment>,
}

impl Weapon {
    pub fn new(name: String, weapon_type: WeaponType) -> Self {
        let stats = match weapon_type {
            WeaponType::Pistol => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::SemiAuto,
                damage: 25.0,
                fire_rate: 300.0,
                magazine_size: 12,
                reload_time: 1.5,
                range: 50.0,
                accuracy: 0.85,
                recoil: 0.3,
                damage_falloff_start: 20.0,
                damage_falloff_end: 50.0,
            },
            WeaponType::Rifle => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::FullAuto,
                damage: 30.0,
                fire_rate: 600.0,
                magazine_size: 30,
                reload_time: 2.5,
                range: 100.0,
                accuracy: 0.75,
                recoil: 0.5,
                damage_falloff_start: 50.0,
                damage_falloff_end: 100.0,
            },
            WeaponType::Shotgun => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::SemiAuto,
                damage: 80.0,
                fire_rate: 60.0,
                magazine_size: 8,
                reload_time: 3.0,
                range: 20.0,
                accuracy: 0.5,
                recoil: 0.9,
                damage_falloff_start: 5.0,
                damage_falloff_end: 20.0,
            },
            WeaponType::Sniper => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::SemiAuto,
                damage: 100.0,
                fire_rate: 40.0,
                magazine_size: 5,
                reload_time: 3.5,
                range: 300.0,
                accuracy: 0.95,
                recoil: 0.8,
                damage_falloff_start: 100.0,
                damage_falloff_end: 300.0,
            },
            WeaponType::SMG => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::FullAuto,
                damage: 20.0,
                fire_rate: 900.0,
                magazine_size: 25,
                reload_time: 2.0,
                range: 30.0,
                accuracy: 0.65,
                recoil: 0.4,
                damage_falloff_start: 10.0,
                damage_falloff_end: 30.0,
            },
            WeaponType::Melee => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::SemiAuto,
                damage: 50.0,
                fire_rate: 120.0,
                magazine_size: 0,
                reload_time: 0.0,
                range: 2.0,
                accuracy: 1.0,
                recoil: 0.0,
                damage_falloff_start: 0.0,
                damage_falloff_end: 2.0,
            },
            WeaponType::Explosive => WeaponStats {
                weapon_type: weapon_type.clone(),
                fire_mode: FireMode::SemiAuto,
                damage: 150.0,
                fire_rate: 30.0,
                magazine_size: 1,
                reload_time: 4.0,
                range: 50.0,
                accuracy: 0.9,
                recoil: 1.0,
                damage_falloff_start: 5.0,
                damage_falloff_end: 15.0,
            },
        };

        Self {
            name,
            stats,
            model_path: PathBuf::new(),
            icon_path: PathBuf::new(),
            attachments: Vec::new(),
        }
    }
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct WeaponEditor {
    weapons: Vec<Weapon>,
    selected_weapon: Option<usize>,
    selected_attachment: Option<usize>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl WeaponEditor {
    pub fn new() -> Self {
        let mut weapons = Vec::new();
        
        // Add example weapons
        weapons.push(Weapon::new("M1911".to_string(), WeaponType::Pistol));
        weapons.push(Weapon::new("M4A1".to_string(), WeaponType::Rifle));
        weapons.push(Weapon::new("SPAS-12".to_string(), WeaponType::Shotgun));
        weapons.push(Weapon::new("AWP".to_string(), WeaponType::Sniper));

        Self {
            weapons,
            selected_weapon: Some(0),
            selected_attachment: None,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🔫 Weapon System Editor");
            ui.separator();
            if ui.button("➕ New Weapon").clicked() {
                self.weapons.push(Weapon::new("New Weapon".to_string(), WeaponType::Pistol));
                self.selected_weapon = Some(self.weapons.len() - 1);
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Left: Weapon list
            ui.vertical(|ui| {
                ui.set_min_width(200.0);
                self.render_weapon_list(ui);
            });

            ui.separator();

            // Right: Weapon properties
            ui.vertical(|ui| {
                ui.set_min_width(500.0);
                self.render_weapon_properties(ui);
            });
        });
    }

    fn render_weapon_list(&mut self, ui: &mut Ui) {
        ui.label("Weapons");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut to_remove = None;
            for (idx, weapon) in self.weapons.iter().enumerate() {
                let is_selected = Some(idx) == self.selected_weapon;
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_selected, &weapon.name).clicked() {
                        self.selected_weapon = Some(idx);
                        self.selected_attachment = None;
                    }
                    if ui.button("🗑️").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }

            if let Some(idx) = to_remove {
                self.weapons.remove(idx);
                if self.selected_weapon == Some(idx) {
                    self.selected_weapon = if self.weapons.is_empty() {
                        None
                    } else {
                        Some(idx.min(self.weapons.len() - 1))
                    };
                }
            }
        });
    }

    fn render_weapon_properties(&mut self, ui: &mut Ui) {
        if let Some(idx) = self.selected_weapon {
            if let Some(weapon) = self.weapons.get_mut(idx) {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut weapon.name);
                    });

                    ui.separator();

                    ui.collapsing("Base Stats", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Type:");
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{:?}", weapon.stats.weapon_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::Pistol, "Pistol");
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::Rifle, "Rifle");
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::Shotgun, "Shotgun");
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::Sniper, "Sniper");
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::SMG, "SMG");
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::Melee, "Melee");
                                    ui.selectable_value(&mut weapon.stats.weapon_type, WeaponType::Explosive, "Explosive");
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Fire Mode:");
                            let current_mode = weapon.stats.fire_mode.clone();
                            match &current_mode {
                                FireMode::SemiAuto => {
                                    if ui.button("Semi-Auto").clicked() {
                                        weapon.stats.fire_mode = FireMode::FullAuto;
                                    }
                                }
                                FireMode::FullAuto => {
                                    if ui.button("Full-Auto").clicked() {
                                        weapon.stats.fire_mode = FireMode::Burst(3);
                                    }
                                }
                                FireMode::Burst(n) => {
                                    if ui.button(format!("Burst ({})", n)).clicked() {
                                        weapon.stats.fire_mode = FireMode::SemiAuto;
                                    }
                                    if let FireMode::Burst(ref mut burst_n) = weapon.stats.fire_mode {
                                        ui.add(egui::DragValue::new(burst_n).range(2..=10));
                                    }
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Damage:");
                            ui.add(egui::DragValue::new(&mut weapon.stats.damage).range(1.0..=500.0).speed(1.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Fire Rate (RPM):");
                            ui.add(egui::DragValue::new(&mut weapon.stats.fire_rate).range(10.0..=1200.0).speed(10.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Magazine Size:");
                            ui.add(egui::DragValue::new(&mut weapon.stats.magazine_size).range(0..=200));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Reload Time (s):");
                            ui.add(egui::DragValue::new(&mut weapon.stats.reload_time).range(0.0..=10.0).speed(0.1));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Range:");
                            ui.add(egui::DragValue::new(&mut weapon.stats.range).range(1.0..=500.0).speed(1.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Accuracy:");
                            ui.add(egui::Slider::new(&mut weapon.stats.accuracy, 0.0..=1.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Recoil:");
                            ui.add(egui::Slider::new(&mut weapon.stats.recoil, 0.0..=2.0));
                        });
                    });

                    ui.separator();

                    ui.collapsing("Damage Falloff", |ui| {
                        ui.label("Damage decreases with distance");
                        ui.horizontal(|ui| {
                            ui.label("Start Distance:");
                            ui.add(egui::DragValue::new(&mut weapon.stats.damage_falloff_start).range(0.0..=500.0).speed(1.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("End Distance:");
                            ui.add(egui::DragValue::new(&mut weapon.stats.damage_falloff_end).range(0.0..=500.0).speed(1.0));
                        });
                    });

                    ui.separator();

                    ui.collapsing("Attachments", |ui| {
                        let mut to_remove = None;
                        for (idx, attachment) in weapon.attachments.iter_mut().enumerate() {
                            let is_selected = Some(idx) == self.selected_attachment;
                            ui.horizontal(|ui| {
                                if ui.selectable_label(is_selected, &attachment.name).clicked() {
                                    self.selected_attachment = Some(idx);
                                }
                                ui.label(format!("({:?})", attachment.attachment_type));
                                if ui.button("🗑️").clicked() {
                                    to_remove = Some(idx);
                                }
                            });

                            if is_selected {
                                ui.indent(idx, |ui| {
                                    ui.text_edit_singleline(&mut attachment.name);
                                    ui.label("Stat Modifiers:");
                                    for modifier in &mut attachment.stat_modifiers {
                                        ui.horizontal(|ui| {
                                            ui.label(&modifier.stat_name);
                                            ui.label(if modifier.is_multiplier { "×" } else { "+" });
                                            ui.add(egui::DragValue::new(&mut modifier.modifier).speed(0.1));
                                        });
                                    }
                                });
                            }
                        }

                        if let Some(idx) = to_remove {
                            weapon.attachments.remove(idx);
                            if self.selected_attachment == Some(idx) {
                                self.selected_attachment = None;
                            }
                        }

                        ui.separator();

                        if ui.button("➕ Add Attachment").clicked() {
                            weapon.attachments.push(WeaponAttachment {
                                name: "New Attachment".to_string(),
                                attachment_type: AttachmentType::Scope,
                                stat_modifiers: vec![
                                    StatModifier {
                                        stat_name: "Accuracy".to_string(),
                                        modifier: 0.1,
                                        is_multiplier: false,
                                    },
                                ],
                            });
                        }
                    });

                    ui.separator();

                    ui.collapsing("Assets", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Model:");
                            ui.label(weapon.model_path.to_string_lossy().to_string());
                            if ui.button("📂").clicked() {
                                // TODO: Open file dialog
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Icon:");
                            ui.label(weapon.icon_path.to_string_lossy().to_string());
                            if ui.button("📂").clicked() {
                                // TODO: Open file dialog
                            }
                        });
                    });
                });
            }
        } else {
            ui.label("No weapon selected");
            ui.separator();
            ui.label("Select a weapon from the list or create a new one");
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for WeaponEditor {
    fn default() -> Self {
        Self::new()
    }
}
