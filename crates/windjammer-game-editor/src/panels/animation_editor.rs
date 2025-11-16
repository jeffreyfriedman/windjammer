// Animation State Machine Editor Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::HashMap;

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug)]
pub struct AnimationState {
    pub id: u32,
    pub name: String,
    pub animation_clip: String,
    pub loop_animation: bool,
    pub speed: f32,
    pub position: Pos2, // For visual editor
}

#[derive(Clone, Debug)]
pub struct AnimationTransition {
    pub from_state: u32,
    pub to_state: u32,
    pub condition: String,
    pub duration: f32,
    pub has_exit_time: bool,
    pub exit_time: f32,
}

#[derive(Clone, Debug)]
pub struct AnimationParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub default_value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParameterType {
    Float,
    Int,
    Bool,
    Trigger,
}

pub struct AnimationStateMachine {
    pub states: HashMap<u32, AnimationState>,
    pub transitions: Vec<AnimationTransition>,
    pub parameters: Vec<AnimationParameter>,
    pub entry_state: Option<u32>,
    next_state_id: u32,
}

impl AnimationStateMachine {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            transitions: Vec::new(),
            parameters: Vec::new(),
            entry_state: None,
            next_state_id: 1,
        }
    }

    pub fn add_state(&mut self, name: String, position: Pos2) -> u32 {
        let id = self.next_state_id;
        self.next_state_id += 1;
        
        self.states.insert(
            id,
            AnimationState {
                id,
                name,
                animation_clip: String::new(),
                loop_animation: true,
                speed: 1.0,
                position,
            },
        );
        
        if self.entry_state.is_none() {
            self.entry_state = Some(id);
        }
        
        id
    }

    pub fn remove_state(&mut self, id: u32) {
        self.states.remove(&id);
        self.transitions.retain(|t| t.from_state != id && t.to_state != id);
        if self.entry_state == Some(id) {
            self.entry_state = self.states.keys().next().copied();
        }
    }

    pub fn add_transition(&mut self, from: u32, to: u32) {
        if from != to && self.states.contains_key(&from) && self.states.contains_key(&to) {
            self.transitions.push(AnimationTransition {
                from_state: from,
                to_state: to,
                condition: String::new(),
                duration: 0.25,
                has_exit_time: false,
                exit_time: 0.75,
            });
        }
    }

    pub fn add_parameter(&mut self, name: String, param_type: ParameterType) {
        self.parameters.push(AnimationParameter {
            name,
            param_type,
            default_value: 0.0,
        });
    }
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AnimationEditor {
    state_machine: AnimationStateMachine,
    selected_state: Option<u32>,
    selected_transition: Option<usize>,
    dragging_state: Option<u32>,
    drag_offset: Vec2,
    creating_transition: Option<u32>,
    zoom: f32,
    pan_offset: Vec2,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AnimationEditor {
    pub fn new() -> Self {
        let mut state_machine = AnimationStateMachine::new();
        
        // Add some example states
        state_machine.add_state("Idle".to_string(), Pos2::new(100.0, 100.0));
        state_machine.add_state("Walk".to_string(), Pos2::new(300.0, 100.0));
        state_machine.add_state("Run".to_string(), Pos2::new(500.0, 100.0));
        state_machine.add_state("Jump".to_string(), Pos2::new(300.0, 250.0));
        
        // Add example transitions
        state_machine.add_transition(1, 2); // Idle -> Walk
        state_machine.add_transition(2, 3); // Walk -> Run
        state_machine.add_transition(2, 4); // Walk -> Jump
        state_machine.add_transition(4, 1); // Jump -> Idle
        
        // Add example parameters
        state_machine.add_parameter("Speed".to_string(), ParameterType::Float);
        state_machine.add_parameter("IsGrounded".to_string(), ParameterType::Bool);
        state_machine.add_parameter("Jump".to_string(), ParameterType::Trigger);
        
        Self {
            state_machine,
            selected_state: None,
            selected_transition: None,
            dragging_state: None,
            drag_offset: Vec2::ZERO,
            creating_transition: None,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🎬 Animation State Machine");
            ui.separator();
            if ui.button("➕ Add State").clicked() {
                let pos = Pos2::new(200.0, 200.0) + self.pan_offset;
                self.state_machine.add_state("New State".to_string(), pos);
            }
            if ui.button("🔗 Add Transition").clicked() && self.selected_state.is_some() {
                self.creating_transition = self.selected_state;
            }
            if ui.button("📊 Add Parameter").clicked() {
                self.state_machine.add_parameter("NewParam".to_string(), ParameterType::Float);
            }
        });

        ui.separator();

        // Split into left panel (graph) and right panel (properties)
        ui.horizontal(|ui| {
            // Left: State machine graph
            ui.vertical(|ui| {
                ui.set_min_width(600.0);
                self.render_state_graph(ui);
            });

            ui.separator();

            // Right: Properties
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                self.render_properties(ui);
            });
        });
    }

    fn render_state_graph(&mut self, ui: &mut Ui) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), 400.0),
            Sense::click_and_drag(),
        );

        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_gray(30));

        // Handle panning
        if response.dragged() && !self.dragging_state.is_some() {
            self.pan_offset += response.drag_delta();
        }

        // Draw transitions first (so they appear behind states)
        for (idx, transition) in self.state_machine.transitions.iter().enumerate() {
            if let (Some(from_state), Some(to_state)) = (
                self.state_machine.states.get(&transition.from_state),
                self.state_machine.states.get(&transition.to_state),
            ) {
                let from_pos = from_state.position + self.pan_offset;
                let to_pos = to_state.position + self.pan_offset;
                
                let color = if Some(idx) == self.selected_transition {
                    Color32::from_rgb(100, 200, 255)
                } else {
                    Color32::from_gray(150)
                };
                
                painter.arrow(from_pos, to_pos - from_pos, Stroke::new(2.0, color));
            }
        }

        // Draw states
        let mut states_to_draw: Vec<_> = self.state_machine.states.values().collect();
        states_to_draw.sort_by_key(|s| s.id);

        for state in states_to_draw {
            let pos = state.position + self.pan_offset;
            let size = Vec2::new(120.0, 60.0);
            let state_rect = Rect::from_min_size(pos - size / 2.0, size);

            let is_selected = Some(state.id) == self.selected_state;
            let is_entry = Some(state.id) == self.state_machine.entry_state;

            // Draw state box
            let fill_color = if is_entry {
                Color32::from_rgb(50, 100, 50)
            } else if is_selected {
                Color32::from_rgb(70, 120, 200)
            } else {
                Color32::from_gray(60)
            };

            painter.rect_filled(state_rect, 5.0, fill_color);
            painter.rect_stroke(
                state_rect,
                5.0,
                Stroke::new(2.0, if is_selected { Color32::WHITE } else { Color32::from_gray(100) }),
            );

            // Draw state name
            painter.text(
                state_rect.center(),
                egui::Align2::CENTER_CENTER,
                &state.name,
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );

            // Handle state interaction
            let state_response = ui.interact(state_rect, ui.id().with(state.id), Sense::click_and_drag());
            
            if state_response.clicked() {
                self.selected_state = Some(state.id);
                self.selected_transition = None;
            }

            if state_response.drag_started() {
                self.dragging_state = Some(state.id);
                self.drag_offset = state_response.interact_pointer_pos().unwrap() - pos;
            }
        }

        // Handle dragging
        if let Some(dragging_id) = self.dragging_state {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(state) = self.state_machine.states.get_mut(&dragging_id) {
                    state.position = pointer_pos - self.drag_offset - self.pan_offset;
                }
            }
            if response.drag_released() {
                self.dragging_state = None;
            }
        }
    }

    fn render_properties(&mut self, ui: &mut Ui) {
        ui.heading("Properties");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // State properties
            if let Some(state_id) = self.selected_state {
                if let Some(state) = self.state_machine.states.get_mut(&state_id) {
                    ui.label("📍 Selected State");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut state.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Animation Clip:");
                        ui.text_edit_singleline(&mut state.animation_clip);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(egui::DragValue::new(&mut state.speed).range(0.0..=10.0).speed(0.1));
                    });

                    ui.checkbox(&mut state.loop_animation, "Loop Animation");

                    ui.separator();

                    if ui.button("🗑️ Delete State").clicked() {
                        let id_to_remove = state_id;
                        self.selected_state = None;
                        self.state_machine.remove_state(id_to_remove);
                    }

                    if ui.button("⭐ Set as Entry State").clicked() {
                        self.state_machine.entry_state = Some(state_id);
                    }
                }
            }

            ui.separator();

            // Parameters
            ui.collapsing("Parameters", |ui| {
                let mut to_remove = None;
                for (idx, param) in self.state_machine.parameters.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut param.name);
                        ui.label(format!("{:?}", param.param_type));
                        if ui.button("🗑️").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                }
                if let Some(idx) = to_remove {
                    self.state_machine.parameters.remove(idx);
                }
            });

            ui.separator();

            // Transitions
            ui.collapsing("Transitions", |ui| {
                for (idx, transition) in self.state_machine.transitions.iter().enumerate() {
                    let from_name = self.state_machine.states.get(&transition.from_state)
                        .map(|s| s.name.as_str()).unwrap_or("?");
                    let to_name = self.state_machine.states.get(&transition.to_state)
                        .map(|s| s.name.as_str()).unwrap_or("?");
                    
                    if ui.button(format!("{} → {}", from_name, to_name)).clicked() {
                        self.selected_transition = Some(idx);
                        self.selected_state = None;
                    }
                }
            });
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AnimationEditor {
    fn default() -> Self {
        Self::new()
    }
}
