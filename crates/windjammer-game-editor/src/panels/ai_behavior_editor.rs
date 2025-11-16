// AI Behavior Tree Visual Editor Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::HashMap;

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum NodeType {
    // Composite nodes
    Sequence,
    Selector,
    Parallel,
    // Decorator nodes
    Inverter,
    Repeater,
    UntilFail,
    // Leaf nodes
    Action(String),
    Condition(String),
}

#[derive(Clone, Debug)]
pub struct BehaviorNode {
    pub id: u32,
    pub node_type: NodeType,
    pub position: Pos2,
    pub children: Vec<u32>,
    pub parent: Option<u32>,
    pub description: String,
}

pub struct BehaviorTree {
    pub nodes: HashMap<u32, BehaviorNode>,
    pub root: Option<u32>,
    next_node_id: u32,
}

impl BehaviorTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            next_node_id: 1,
        }
    }

    pub fn add_node(&mut self, node_type: NodeType, position: Pos2) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;

        self.nodes.insert(
            id,
            BehaviorNode {
                id,
                node_type,
                position,
                children: Vec::new(),
                parent: None,
                description: String::new(),
            },
        );

        if self.root.is_none() {
            self.root = Some(id);
        }

        id
    }

    pub fn remove_node(&mut self, id: u32) {
        if let Some(node) = self.nodes.remove(&id) {
            // Remove from parent's children
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|&child_id| child_id != id);
                }
            }

            // Remove all children recursively
            for child_id in node.children {
                self.remove_node(child_id);
            }

            // Update root if necessary
            if self.root == Some(id) {
                self.root = self.nodes.keys().next().copied();
            }
        }
    }

    pub fn add_child(&mut self, parent_id: u32, child_id: u32) {
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if !parent.children.contains(&child_id) {
                parent.children.push(child_id);
            }
        }
        if let Some(child) = self.nodes.get_mut(&child_id) {
            child.parent = Some(parent_id);
        }
    }

    pub fn can_have_children(&self, node_id: u32) -> bool {
        if let Some(node) = self.nodes.get(&node_id) {
            matches!(
                node.node_type,
                NodeType::Sequence
                    | NodeType::Selector
                    | NodeType::Parallel
                    | NodeType::Inverter
                    | NodeType::Repeater
                    | NodeType::UntilFail
            )
        } else {
            false
        }
    }
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AIBehaviorEditor {
    tree: BehaviorTree,
    selected_node: Option<u32>,
    dragging_node: Option<u32>,
    drag_offset: Vec2,
    connecting_from: Option<u32>,
    zoom: f32,
    pan_offset: Vec2,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AIBehaviorEditor {
    pub fn new() -> Self {
        let mut tree = BehaviorTree::new();

        // Create example tree
        let root = tree.add_node(NodeType::Sequence, Pos2::new(400.0, 50.0));
        let check_enemy = tree.add_node(NodeType::Condition("Enemy in range".to_string()), Pos2::new(200.0, 150.0));
        let attack_sequence = tree.add_node(NodeType::Sequence, Pos2::new(400.0, 150.0));
        let patrol = tree.add_node(NodeType::Action("Patrol".to_string()), Pos2::new(600.0, 150.0));

        let aim = tree.add_node(NodeType::Action("Aim at enemy".to_string()), Pos2::new(300.0, 250.0));
        let shoot = tree.add_node(NodeType::Action("Shoot".to_string()), Pos2::new(500.0, 250.0));

        tree.add_child(root, check_enemy);
        tree.add_child(root, attack_sequence);
        tree.add_child(root, patrol);
        tree.add_child(attack_sequence, aim);
        tree.add_child(attack_sequence, shoot);

        tree.root = Some(root);

        Self {
            tree,
            selected_node: None,
            dragging_node: None,
            drag_offset: Vec2::ZERO,
            connecting_from: None,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🤖 AI Behavior Tree");
            ui.separator();

            ui.menu_button("➕ Add Node", |ui| {
                if ui.button("Sequence").clicked() {
                    self.tree.add_node(NodeType::Sequence, Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
                if ui.button("Selector").clicked() {
                    self.tree.add_node(NodeType::Selector, Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
                if ui.button("Parallel").clicked() {
                    self.tree.add_node(NodeType::Parallel, Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Inverter").clicked() {
                    self.tree.add_node(NodeType::Inverter, Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
                if ui.button("Repeater").clicked() {
                    self.tree.add_node(NodeType::Repeater, Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Action").clicked() {
                    self.tree.add_node(NodeType::Action("New Action".to_string()), Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
                if ui.button("Condition").clicked() {
                    self.tree.add_node(NodeType::Condition("New Condition".to_string()), Pos2::new(400.0, 200.0) + self.pan_offset);
                    ui.close_menu();
                }
            });

            if ui.button("🔗 Connect").clicked() && self.selected_node.is_some() {
                self.connecting_from = self.selected_node;
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Left: Tree view
            ui.vertical(|ui| {
                ui.set_min_width(600.0);
                self.render_tree_view(ui);
            });

            ui.separator();

            // Right: Properties
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                self.render_properties(ui);
            });
        });
    }

    fn render_tree_view(&mut self, ui: &mut Ui) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), 500.0),
            Sense::click_and_drag(),
        );

        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_gray(25));

        // Handle panning
        if response.dragged() && self.dragging_node.is_none() {
            self.pan_offset += response.drag_delta();
        }

        // Draw connections first
        for node in self.tree.nodes.values() {
            let parent_pos = node.position + self.pan_offset;

            for &child_id in &node.children {
                if let Some(child) = self.tree.nodes.get(&child_id) {
                    let child_pos = child.position + self.pan_offset;
                    painter.line_segment(
                        [parent_pos + Vec2::new(0.0, 30.0), child_pos - Vec2::new(0.0, 30.0)],
                        Stroke::new(2.0, Color32::from_gray(150)),
                    );
                }
            }
        }

        // Draw nodes
        let mut node_ids: Vec<_> = self.tree.nodes.keys().copied().collect();
        node_ids.sort();

        for node_id in node_ids {
            let node = match self.tree.nodes.get(&node_id) {
                Some(n) => n,
                None => continue,
            };
            let pos = node.position + self.pan_offset;
            let size = Vec2::new(140.0, 60.0);
            let node_rect = Rect::from_center_size(pos, size);

            let is_selected = Some(node.id) == self.selected_node;
            let is_root = Some(node.id) == self.tree.root;

            // Determine node color based on type
            let fill_color = match &node.node_type {
                NodeType::Sequence => Color32::from_rgb(70, 120, 200),
                NodeType::Selector => Color32::from_rgb(200, 120, 70),
                NodeType::Parallel => Color32::from_rgb(120, 70, 200),
                NodeType::Inverter | NodeType::Repeater | NodeType::UntilFail => Color32::from_rgb(150, 150, 70),
                NodeType::Action(_) => Color32::from_rgb(70, 150, 70),
                NodeType::Condition(_) => Color32::from_rgb(200, 200, 70),
            };

            let final_color = if is_selected {
                Color32::from_rgb(
                    fill_color.r().saturating_add(30),
                    fill_color.g().saturating_add(30),
                    fill_color.b().saturating_add(30),
                )
            } else {
                fill_color
            };

            painter.rect_filled(node_rect, 5.0, final_color);
            
            if is_root {
                painter.rect_stroke(node_rect, 5.0, Stroke::new(3.0, Color32::from_rgb(255, 215, 0)));
            } else {
                painter.rect_stroke(node_rect, 5.0, Stroke::new(1.0, Color32::from_gray(100)));
            }

            // Draw node label
            let label = match &node.node_type {
                NodeType::Sequence => "→ Sequence",
                NodeType::Selector => "? Selector",
                NodeType::Parallel => "⚡ Parallel",
                NodeType::Inverter => "! Inverter",
                NodeType::Repeater => "🔁 Repeater",
                NodeType::UntilFail => "⏳ Until Fail",
                NodeType::Action(name) => name.as_str(),
                NodeType::Condition(name) => name.as_str(),
            };

            painter.text(
                node_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );

            // Handle node interaction
            let current_node_id = node.id;
            let node_response = ui.interact(node_rect, ui.id().with(current_node_id), Sense::click_and_drag());

            if node_response.clicked() {
                if let Some(from_id) = self.connecting_from {
                    if from_id != current_node_id && self.tree.can_have_children(from_id) {
                        self.tree.add_child(from_id, current_node_id);
                        self.connecting_from = None;
                    }
                } else {
                    self.selected_node = Some(current_node_id);
                }
            }

            if node_response.drag_started() {
                self.dragging_node = Some(current_node_id);
                self.drag_offset = node_response.interact_pointer_pos().unwrap() - pos;
            }
        }

        // Handle dragging
        if let Some(dragging_id) = self.dragging_node {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(node) = self.tree.nodes.get_mut(&dragging_id) {
                    node.position = pointer_pos - self.drag_offset - self.pan_offset;
                }
            }
            if response.drag_released() {
                self.dragging_node = None;
            }
        }

        // Show connection mode indicator
        if self.connecting_from.is_some() {
            painter.text(
                rect.min + Vec2::new(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                "🔗 Click a node to connect",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(255, 255, 0),
            );
        }
    }

    fn render_properties(&mut self, ui: &mut Ui) {
        ui.heading("Properties");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(node_id) = self.selected_node {
                let mut should_delete = false;
                let mut should_set_root = false;
                
                if let Some(node) = self.tree.nodes.get_mut(&node_id) {
                    ui.label("📍 Selected Node");
                    ui.separator();

                    ui.label(format!("Type: {:?}", node.node_type));

                    match &mut node.node_type {
                        NodeType::Action(name) | NodeType::Condition(name) => {
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(name);
                            });
                        }
                        _ => {}
                    }

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_multiline(&mut node.description);
                    });

                    ui.separator();

                    if ui.button("🗑️ Delete Node").clicked() {
                        should_delete = true;
                    }

                    if ui.button("⭐ Set as Root").clicked() {
                        should_set_root = true;
                    }

                    ui.separator();

                    ui.label(format!("Children: {}", node.children.len()));
                    let children_to_show = node.children.clone();
                    for child_id in children_to_show {
                        ui.horizontal(|ui| {
                            if let Some(child) = self.tree.nodes.get(&child_id) {
                                let child_name = match &child.node_type {
                                    NodeType::Action(name) | NodeType::Condition(name) => name.clone(),
                                    _ => format!("{:?}", child.node_type),
                                };
                                ui.label(format!("  • {}", child_name));
                                if ui.button("❌").clicked() {
                                    if let Some(node) = self.tree.nodes.get_mut(&node_id) {
                                        node.children.retain(|&id| id != child_id);
                                    }
                                    if let Some(child) = self.tree.nodes.get_mut(&child_id) {
                                        child.parent = None;
                                    }
                                }
                            }
                        });
                    }
                }
                
                // Handle deferred actions
                if should_delete {
                    self.selected_node = None;
                    self.tree.remove_node(node_id);
                }
                if should_set_root {
                    self.tree.root = Some(node_id);
                }
            } else {
                ui.label("No node selected");
                ui.separator();
                ui.label("Click a node to view its properties");
            }

            ui.separator();

            ui.collapsing("Node Types", |ui| {
                ui.label("Composite Nodes:");
                ui.label("  • Sequence: Execute children in order until one fails");
                ui.label("  • Selector: Execute children until one succeeds");
                ui.label("  • Parallel: Execute all children simultaneously");
                ui.separator();
                ui.label("Decorator Nodes:");
                ui.label("  • Inverter: Invert child result");
                ui.label("  • Repeater: Repeat child N times");
                ui.separator();
                ui.label("Leaf Nodes:");
                ui.label("  • Action: Perform an action");
                ui.label("  • Condition: Check a condition");
            });
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AIBehaviorEditor {
    fn default() -> Self {
        Self::new()
    }
}
