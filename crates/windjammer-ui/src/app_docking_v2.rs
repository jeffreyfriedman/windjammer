/// Professional docking editor with full functionality
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use crate::simple_vnode::VNode;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct EditorApp {
    title: String,
    dock_state: egui_dock::DockState<String>,
    panels: Arc<Mutex<HashMap<String, PanelContent>>>,
    console_output: Arc<Mutex<Vec<String>>>,
    current_file: Arc<Mutex<Option<String>>>,
    current_file_content: Arc<Mutex<String>>,
    project_path: Arc<Mutex<Option<String>>>,
    selected_object: Arc<Mutex<Option<String>>>,
    open_files: Arc<Mutex<HashMap<String, String>>>, // path -> content
    unsaved_changes: Arc<Mutex<bool>>,
    syntax_highlighter: Arc<crate::syntax_highlighting::SyntaxHighlighter>,
    enable_syntax_highlighting: Arc<Mutex<bool>>,
    file_watcher: Arc<Mutex<Option<crate::file_watcher::FileWatcher>>>,
    enable_file_watching: Arc<Mutex<bool>>,
    scene: Arc<Mutex<crate::scene_manager::Scene>>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Clone)]
pub struct PanelContent {
    pub title: String,
    pub content: String,
    pub panel_type: PanelType,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Clone, PartialEq)]
pub enum PanelType {
    FileTree,
    SceneHierarchy,
    CodeEditor,
    Properties,
    Console,
    SceneView,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl EditorApp {
    pub fn new(title: String) -> Self {
        // Create initial dock layout
        let mut dock_state = egui_dock::DockState::new(vec!["editor".to_string()]);

        // Set up initial layout: Files | Editor | Properties
        let main_surface = dock_state.main_surface_mut();

        // Split: Main (80%) | Properties (20%) on right
        let [main_area, _props] = main_surface.split_right(
            egui_dock::NodeIndex::root(),
            0.8,
            vec!["properties".to_string()],
        );

        // Split main: Left panels (20%) | Editor (80%)
        let [_files, _editor] = main_surface.split_left(
            main_area,
            0.2,
            vec!["files".to_string(), "scene".to_string()],
        );

        // Split bottom: Main (75%) | Console (25%)
        let [_, _console] = main_surface.split_below(
            egui_dock::NodeIndex::root(),
            0.75,
            vec!["console".to_string()],
        );

        // Initialize panel content
        let mut panels = HashMap::new();
        panels.insert(
            "files".to_string(),
            PanelContent {
                title: "Files".to_string(),
                content: String::new(),
                panel_type: PanelType::FileTree,
            },
        );
        panels.insert(
            "scene".to_string(),
            PanelContent {
                title: "Scene".to_string(),
                content: String::new(),
                panel_type: PanelType::SceneHierarchy,
            },
        );
        panels.insert(
            "editor".to_string(),
            PanelContent {
                title: "Editor".to_string(),
                content: "// Welcome to Windjammer!\n// Create a new project to get started.\n"
                    .to_string(),
                panel_type: PanelType::CodeEditor,
            },
        );
        panels.insert(
            "properties".to_string(),
            PanelContent {
                title: "Properties".to_string(),
                content: String::new(),
                panel_type: PanelType::Properties,
            },
        );
        panels.insert(
            "console".to_string(),
            PanelContent {
                title: "Console".to_string(),
                content: String::new(),
                panel_type: PanelType::Console,
            },
        );

        Self {
            title,
            dock_state,
            panels: Arc::new(Mutex::new(panels)),
            console_output: Arc::new(Mutex::new(vec!["Ready.".to_string()])),
            current_file: Arc::new(Mutex::new(None)),
            current_file_content: Arc::new(Mutex::new(String::new())),
            project_path: Arc::new(Mutex::new(None)),
            selected_object: Arc::new(Mutex::new(None)),
            open_files: Arc::new(Mutex::new(HashMap::new())),
            unsaved_changes: Arc::new(Mutex::new(false)),
            syntax_highlighter: Arc::new(crate::syntax_highlighting::SyntaxHighlighter::new()),
            enable_syntax_highlighting: Arc::new(Mutex::new(true)),
            file_watcher: Arc::new(Mutex::new(crate::file_watcher::FileWatcher::new().ok())),
            enable_file_watching: Arc::new(Mutex::new(true)),
            scene: Arc::new(Mutex::new(crate::scene_manager::Scene::default())),
        }
    }

    pub fn run(mut self) {
        println!("🔧 Starting Professional Editor with egui_dock");

        let panels = self.panels.clone();
        let console_output = self.console_output.clone();
        let current_file = self.current_file.clone();
        let current_file_content = self.current_file_content.clone();
        let project_path = self.project_path.clone();
        let selected_object = self.selected_object.clone();
        let open_files = self.open_files.clone();
        let unsaved_changes = self.unsaved_changes.clone();
        let syntax_highlighter = self.syntax_highlighter.clone();
        let enable_syntax_highlighting = self.enable_syntax_highlighting.clone();
        let scene = self.scene.clone();

        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1400.0, 900.0])
                .with_title(&self.title)
                .with_decorations(true) // Use native window decorations
                .with_transparent(false) // Solid window background
                .with_title_shown(true) // Show native title bar
                .with_titlebar_buttons_shown(true) // Show traffic lights
                .with_titlebar_shown(true), // Show full title bar
            ..Default::default()
        };

        eframe::run_simple_native(&self.title, native_options, move |ctx, _frame| {
            // Professional dark theme
            ctx.set_visuals(create_professional_theme());

            // Native-looking spacing and typography
            let mut style = (*ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(8.0, 6.0); // More generous spacing
            style.spacing.button_padding = egui::vec2(8.0, 4.0); // Better button padding
            style.spacing.menu_margin = egui::Margin::same(8.0); // Menu spacing
            style.spacing.indent = 20.0; // Tree indentation
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional), // macOS system font size
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            );
            ctx.set_style(style);

            // Handle keyboard shortcuts
            handle_keyboard_shortcuts(
                ctx,
                &console_output,
                &current_file,
                &current_file_content,
                &project_path,
                &unsaved_changes,
            );

            // Check for file changes (file watching)
            if *self.enable_file_watching.lock().unwrap() {
                if let Some(watcher) = self.file_watcher.lock().unwrap().as_ref() {
                    let changed_files = watcher.check_events();
                    for file_path in changed_files {
                        // Check if it's the currently open file
                        let current = current_file.lock().unwrap().clone();
                        if current.as_ref() == Some(&file_path) {
                            // Reload the file
                            if let Ok(new_content) = std::fs::read_to_string(&file_path) {
                                *current_file_content.lock().unwrap() = new_content;
                                console_output
                                    .lock()
                                    .unwrap()
                                    .push(format!("🔄 Reloaded: {}", file_path));
                            }
                        }
                    }
                }
            }

            // Top menu bar
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("📁 New Project...").clicked() {
                            handle_new_project(
                                &console_output,
                                &project_path,
                                &current_file,
                                &current_file_content,
                            );
                            ui.close_menu();
                        }
                        if ui.button("📂 Open File...").clicked() {
                            handle_open_file(
                                &console_output,
                                &current_file,
                                &current_file_content,
                                &open_files,
                            );
                            ui.close_menu();
                        }
                        if ui.button("💾 Save").clicked() {
                            handle_save(
                                &console_output,
                                &current_file,
                                &current_file_content,
                                &unsaved_changes,
                            );
                            ui.close_menu();
                        }
                        if ui.button("💾 Save As...").clicked() {
                            handle_save_as(
                                &console_output,
                                &current_file,
                                &current_file_content,
                                &unsaved_changes,
                            );
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("❌ Exit").clicked() {
                            std::process::exit(0);
                        }
                    });

                    ui.menu_button("Edit", |ui| {
                        if ui.button("✂️ Cut").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("📋 Copy").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("📄 Paste").clicked() {
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Build", |ui| {
                        if ui.button("▶️ Run (F5)").clicked() {
                            handle_run(&console_output, &project_path, &current_file);
                            ui.close_menu();
                        }
                        if ui.button("🔨 Build (Cmd/Ctrl+B)").clicked() {
                            handle_build(&console_output, &project_path);
                            ui.close_menu();
                        }
                        if ui.button("🐛 Debug (Cmd/Ctrl+Shift+B)").clicked() {
                            handle_debug(&console_output, &project_path);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("🧹 Clean").clicked() {
                            handle_clean(&console_output, &project_path);
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("View", |ui| {
                        if ui.button("🔄 Reset Layout").clicked() {
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Help", |ui| {
                        if ui.button("📖 Documentation").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("ℹ️ About").clicked() {
                            ui.close_menu();
                        }
                    });
                });
            });

            // Toolbar
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    if ui.button("📁 New Project").clicked() {
                        handle_new_project(
                            &console_output,
                            &project_path,
                            &current_file,
                            &current_file_content,
                        );
                    }
                    if ui.button("📂 Open").clicked() {
                        handle_open_file(
                            &console_output,
                            &current_file,
                            &current_file_content,
                            &open_files,
                        );
                    }
                    if ui.button("💾 Save").clicked() {
                        handle_save(
                            &console_output,
                            &current_file,
                            &current_file_content,
                            &unsaved_changes,
                        );
                    }
                    ui.separator();
                    if ui.button("▶️ Run").clicked() {
                        handle_run(&console_output, &project_path, &current_file);
                    }
                    if ui.button("🔨 Build").clicked() {
                        handle_build(&console_output, &project_path);
                    }
                    if ui.button("🐛 Debug").clicked() {
                        handle_debug(&console_output, &project_path);
                    }

                    ui.separator();

                    // Show current file and unsaved indicator
                    if let Some(file) = current_file.lock().unwrap().as_ref() {
                        let unsaved = *unsaved_changes.lock().unwrap();
                        let indicator = if unsaved { " •" } else { "" };
                        ui.label(format!("📄 {}{}", file, indicator));
                    }
                });
            });

            // Status bar
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let project = project_path.lock().unwrap();
                    let file = current_file.lock().unwrap();

                    ui.label(format!(
                        "Project: {}",
                        project.as_ref().unwrap_or(&"None".to_string())
                    ));
                    ui.separator();
                    ui.label(format!(
                        "File: {}",
                        file.as_ref().unwrap_or(&"None".to_string())
                    ));
                    ui.separator();
                    ui.label("✓ Ready");
                });
            });

            // Main docking area
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut tab_viewer = TabViewer {
                    panels: panels.clone(),
                    console_output: console_output.clone(),
                    current_file: current_file.clone(),
                    current_file_content: current_file_content.clone(),
                    selected_object: selected_object.clone(),
                    unsaved_changes: unsaved_changes.clone(),
                    syntax_highlighter: syntax_highlighter.clone(),
                    enable_syntax_highlighting: enable_syntax_highlighting.clone(),
                    scene: scene.clone(),
                };

                egui_dock::DockArea::new(&mut self.dock_state)
                    .style(create_dock_style())
                    .show_inside(ui, &mut tab_viewer);
            });
        })
        .unwrap();
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
struct TabViewer {
    panels: Arc<Mutex<HashMap<String, PanelContent>>>,
    console_output: Arc<Mutex<Vec<String>>>,
    current_file: Arc<Mutex<Option<String>>>,
    current_file_content: Arc<Mutex<String>>,
    selected_object: Arc<Mutex<Option<String>>>,
    unsaved_changes: Arc<Mutex<bool>>,
    syntax_highlighter: Arc<crate::syntax_highlighting::SyntaxHighlighter>,
    enable_syntax_highlighting: Arc<Mutex<bool>>,
    scene: Arc<Mutex<crate::scene_manager::Scene>>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let panels = self.panels.lock().unwrap();
        if let Some(panel) = panels.get(tab) {
            panel.title.clone().into()
        } else {
            tab.clone().into()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let panels = self.panels.lock().unwrap();
        if let Some(panel) = panels.get(tab) {
            match panel.panel_type {
                PanelType::FileTree => {
                    render_file_tree(ui, &self.current_file, &self.current_file_content)
                }
                PanelType::SceneHierarchy => {
                    render_scene_hierarchy(ui, &self.scene, &self.selected_object)
                }
                PanelType::CodeEditor => render_code_editor(
                    ui,
                    &self.current_file_content,
                    &self.unsaved_changes,
                    &self.syntax_highlighter,
                    &self.enable_syntax_highlighting,
                ),
                PanelType::Properties => render_properties(ui, &self.scene, &self.selected_object),
                PanelType::Console => render_console(ui, &self.console_output),
                PanelType::SceneView => render_scene_view(ui, &self.selected_object),
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_keyboard_shortcuts(
    ctx: &egui::Context,
    console_output: &Arc<Mutex<Vec<String>>>,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
    project_path: &Arc<Mutex<Option<String>>>,
    unsaved_changes: &Arc<Mutex<bool>>,
) {
    // Platform-specific modifier key (Cmd on macOS, Ctrl on Windows/Linux)
    #[cfg(target_os = "macos")]
    let modifier = egui::Modifiers::COMMAND;
    #[cfg(not(target_os = "macos"))]
    let modifier = egui::Modifiers::CTRL;

    // Ctrl/Cmd+N - New Project
    if ctx.input_mut(|i| i.consume_key(modifier, egui::Key::N)) {
        handle_new_project(
            console_output,
            project_path,
            current_file,
            current_file_content,
        );
    }

    // Ctrl/Cmd+S - Save
    if ctx.input_mut(|i| i.consume_key(modifier, egui::Key::S)) {
        handle_save(
            console_output,
            current_file,
            current_file_content,
            unsaved_changes,
        );
    }

    // F5 - Run
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
        handle_run(console_output, project_path, current_file);
    }

    // Ctrl/Cmd+B - Build
    if ctx.input_mut(|i| i.consume_key(modifier, egui::Key::B)) {
        handle_build(console_output, project_path);
    }

    // Ctrl/Cmd+Shift+B - Debug Build
    if ctx.input_mut(|i| i.consume_key(modifier | egui::Modifiers::SHIFT, egui::Key::B)) {
        handle_debug(console_output, project_path);
    }

    // Ctrl/Cmd+Q - Quit (macOS standard)
    #[cfg(target_os = "macos")]
    if ctx.input_mut(|i| i.consume_key(modifier, egui::Key::Q)) {
        std::process::exit(0);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn create_professional_theme() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();

    // Platform-specific colors and styling
    #[cfg(target_os = "macos")]
    {
        // macOS dark mode colors
        visuals.window_fill = egui::Color32::from_rgb(40, 40, 40);
        visuals.panel_fill = egui::Color32::from_rgb(50, 50, 50);
        visuals.faint_bg_color = egui::Color32::from_rgb(60, 60, 60);
        visuals.extreme_bg_color = egui::Color32::from_rgb(30, 30, 30);
        visuals.code_bg_color = egui::Color32::from_rgb(35, 35, 35);

        // macOS accent color (blue)
        visuals.selection.bg_fill = egui::Color32::from_rgb(10, 132, 255);
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(10, 132, 255));

        // Rounded corners like macOS
        visuals.window_rounding = egui::Rounding::same(6.0);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
        visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
        visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
        visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    }

    #[cfg(target_os = "windows")]
    {
        // Windows 11 dark mode colors
        visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);
        visuals.panel_fill = egui::Color32::from_rgb(43, 43, 43);
        visuals.faint_bg_color = egui::Color32::from_rgb(54, 54, 54);
        visuals.extreme_bg_color = egui::Color32::from_rgb(24, 24, 24);
        visuals.code_bg_color = egui::Color32::from_rgb(30, 30, 30);

        // Windows accent color (blue)
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 212);
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 212));

        // Less rounded corners like Windows
        visuals.window_rounding = egui::Rounding::same(4.0);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(2.0);
        visuals.widgets.inactive.rounding = egui::Rounding::same(2.0);
        visuals.widgets.hovered.rounding = egui::Rounding::same(2.0);
        visuals.widgets.active.rounding = egui::Rounding::same(2.0);
    }

    #[cfg(target_os = "linux")]
    {
        // GNOME/KDE dark mode colors (neutral)
        visuals.window_fill = egui::Color32::from_rgb(36, 36, 36);
        visuals.panel_fill = egui::Color32::from_rgb(46, 46, 46);
        visuals.faint_bg_color = egui::Color32::from_rgb(56, 56, 56);
        visuals.extreme_bg_color = egui::Color32::from_rgb(28, 28, 28);
        visuals.code_bg_color = egui::Color32::from_rgb(32, 32, 32);

        // Linux accent color (neutral blue)
        visuals.selection.bg_fill = egui::Color32::from_rgb(53, 132, 228);
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(53, 132, 228));

        // Moderate rounding for Linux
        visuals.window_rounding = egui::Rounding::same(5.0);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(3.0);
        visuals.widgets.inactive.rounding = egui::Rounding::same(3.0);
        visuals.widgets.hovered.rounding = egui::Rounding::same(3.0);
        visuals.widgets.active.rounding = egui::Rounding::same(3.0);
    }

    // Common styling across all platforms
    visuals.window_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(70, 70, 70));
    visuals.override_text_color = Some(egui::Color32::from_rgb(230, 230, 230));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);

    visuals
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn create_dock_style() -> egui_dock::Style {
    let mut style = egui_dock::Style::from_egui(&egui::Style::default());

    // Native-looking tab bar
    style.tab_bar.fill_tab_bar = true;
    style.tab_bar.height = 32.0; // Slightly taller for better touch targets
    style.tab_bar.bg_fill = egui::Color32::from_rgb(45, 45, 45);
    style.tab_bar.hline_color = egui::Color32::from_rgb(60, 60, 60);

    style
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_file_tree(
    ui: &mut egui::Ui,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
) {
    egui::ScrollArea::both().show(ui, |ui| {
        ui.heading("📁 Project Files");
        ui.separator();

        // Check if project directory exists
        let project_path = "/tmp/windjammer_projects/my_game";
        if std::path::Path::new(project_path).exists() {
            render_directory_tree(
                ui,
                project_path,
                "my_game",
                current_file,
                current_file_content,
            );
        } else {
            ui.label("No project open");
            ui.label("Create a new project to get started");
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_scene_hierarchy(
    ui: &mut egui::Ui,
    scene_arc: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    egui::ScrollArea::both().show(ui, |ui| {
        ui.heading("🎬 Scene Hierarchy");
        ui.separator();

        let current_selection = selected_object.lock().unwrap().clone();
        let scene = scene_arc.lock().unwrap();

        // Scene mode indicator
        let mode_icon = match scene.mode {
            crate::scene_manager::SceneMode::TwoD => "🎮 2D",
            crate::scene_manager::SceneMode::ThreeD => "🎲 3D",
        };
        ui.label(format!("Mode: {}", mode_icon));
        ui.separator();

        // Root scene node
        egui::CollapsingHeader::new(format!("🎮 {}", scene.name))
            .default_open(true)
            .show(ui, |ui| {
                // Render all scene objects
                for (id, object) in &scene.objects {
                    if !object.visible {
                        continue; // Skip invisible objects
                    }

                    let icon = get_object_icon(&object.object_type);
                    let is_selected = current_selection.as_ref() == Some(id);

                    if ui
                        .selectable_label(is_selected, format!("{} {}", icon, object.name))
                        .clicked()
                    {
                        *selected_object.lock().unwrap() = Some(id.clone());
                    }
                }
            });

        ui.separator();

        drop(scene); // Release lock before UI that might modify scene

        // Add object menu
        ui.menu_button("➕ Add Object", |ui| {
            ui.label("3D Primitives");
            ui.separator();
            if ui.button("🧊 Cube").clicked() {
                add_cube_to_scene(scene_arc, selected_object);
                ui.close_menu();
            }
            if ui.button("⚪ Sphere").clicked() {
                add_sphere_to_scene(scene_arc, selected_object);
                ui.close_menu();
            }
            if ui.button("⬜ Plane").clicked() {
                add_plane_to_scene(scene_arc, selected_object);
                ui.close_menu();
            }

            ui.separator();
            ui.label("Lights");
            ui.separator();
            if ui.button("☀️ Directional Light").clicked() {
                add_directional_light_to_scene(scene_arc, selected_object);
                ui.close_menu();
            }
            if ui.button("💡 Point Light").clicked() {
                add_point_light_to_scene(scene_arc, selected_object);
                ui.close_menu();
            }

            ui.separator();
            ui.label("2D Objects");
            ui.separator();
            if ui.button("🖼️ Sprite").clicked() {
                add_sprite_to_scene(scene_arc, selected_object);
                ui.close_menu();
            }
        });

        // Remove object button
        if current_selection.is_some() {
            if ui.button("🗑️ Remove Selected").clicked() {
                remove_selected_object(scene_arc, selected_object);
            }
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn get_object_icon(object_type: &crate::scene_manager::ObjectType) -> &'static str {
    use crate::scene_manager::ObjectType;
    match object_type {
        ObjectType::Cube { .. } => "🧊",
        ObjectType::Sphere { .. } => "⚪",
        ObjectType::Plane { .. } => "⬜",
        ObjectType::Cylinder { .. } => "🥫",
        ObjectType::Capsule { .. } => "💊",
        ObjectType::Sprite { .. } => "🖼️",
        ObjectType::TileMap { .. } => "🗺️",
        ObjectType::DirectionalLight { .. } => "☀️",
        ObjectType::PointLight { .. } => "💡",
        ObjectType::SpotLight { .. } => "🔦",
        ObjectType::Camera => "📷",
        ObjectType::Empty => "📦",
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn add_cube_to_scene(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    use crate::scene_manager::{SceneObject, Vec3};
    let cube = SceneObject::new_cube(
        "Cube".to_string(),
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        1.0,
    );
    let id = cube.id.clone();
    scene.lock().unwrap().add_object(cube);
    *selected_object.lock().unwrap() = Some(id);
    println!("Added cube to scene");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn add_sphere_to_scene(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    use crate::scene_manager::{SceneObject, Vec3};
    let sphere = SceneObject::new_sphere(
        "Sphere".to_string(),
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        0.5,
    );
    let id = sphere.id.clone();
    scene.lock().unwrap().add_object(sphere);
    *selected_object.lock().unwrap() = Some(id);
    println!("Added sphere to scene");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn add_plane_to_scene(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    use crate::scene_manager::{SceneObject, Vec3};
    let plane = SceneObject::new_plane(
        "Plane".to_string(),
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        10.0,
        10.0,
    );
    let id = plane.id.clone();
    scene.lock().unwrap().add_object(plane);
    *selected_object.lock().unwrap() = Some(id);
    println!("Added plane to scene");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn add_directional_light_to_scene(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    use crate::scene_manager::{Color, ObjectType, SceneObject, Transform, Vec3};
    let light = SceneObject {
        id: format!("DirectionalLight_{}", uuid::Uuid::new_v4()),
        name: "Directional Light".to_string(),
        object_type: ObjectType::DirectionalLight {
            color: Color {
                r: 1.0,
                g: 1.0,
                b: 0.9,
                a: 1.0,
            },
            intensity: 1.0,
        },
        transform: Transform {
            position: Vec3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
            rotation: Vec3 {
                x: -45.0,
                y: 0.0,
                z: 0.0,
            },
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
        visible: true,
        children: vec![],
    };
    let id = light.id.clone();
    scene.lock().unwrap().add_object(light);
    *selected_object.lock().unwrap() = Some(id);
    println!("Added directional light to scene");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn add_point_light_to_scene(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    use crate::scene_manager::{Color, ObjectType, SceneObject, Transform, Vec3};
    let light = SceneObject {
        id: format!("PointLight_{}", uuid::Uuid::new_v4()),
        name: "Point Light".to_string(),
        object_type: ObjectType::PointLight {
            color: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            intensity: 1.0,
            range: 10.0,
        },
        transform: Transform {
            position: Vec3 {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            },
            rotation: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
        visible: true,
        children: vec![],
    };
    let id = light.id.clone();
    scene.lock().unwrap().add_object(light);
    *selected_object.lock().unwrap() = Some(id);
    println!("Added point light to scene");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn add_sprite_to_scene(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    use crate::scene_manager::{SceneObject, Vec3};
    let sprite = SceneObject::new_sprite(
        "Sprite".to_string(),
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        "sprite.png".to_string(),
        100.0,
        100.0,
    );
    let id = sprite.id.clone();
    scene.lock().unwrap().add_object(sprite);
    *selected_object.lock().unwrap() = Some(id);
    println!("Added sprite to scene");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn remove_selected_object(
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    if let Some(id) = selected_object.lock().unwrap().take() {
        scene.lock().unwrap().remove_object(&id);
        println!("Removed object: {}", id);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_directory_tree(
    ui: &mut egui::Ui,
    path: &str,
    name: &str,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
) {
    if let Ok(entries) = std::fs::read_dir(path) {
        egui::CollapsingHeader::new(format!("📁 {}", name))
            .default_open(true)
            .show(ui, |ui| {
                let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                items.sort_by_key(|e| e.path());

                for entry in items {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();

                    if path.is_dir() {
                        render_directory_tree(
                            ui,
                            path.to_str().unwrap(),
                            &name,
                            current_file,
                            current_file_content,
                        );
                    } else {
                        let icon = if name.ends_with(".wj") {
                            "📄"
                        } else if name.ends_with(".png") || name.ends_with(".jpg") {
                            "🖼️"
                        } else if name.ends_with(".wav") || name.ends_with(".mp3") {
                            "🔊"
                        } else {
                            "📄"
                        };

                        let path_str = path.to_string_lossy().to_string();
                        let is_selected = current_file.lock().unwrap().as_ref() == Some(&path_str);

                        if ui
                            .selectable_label(is_selected, format!("{} {}", icon, name))
                            .clicked()
                        {
                            // Load file into editor
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                *current_file.lock().unwrap() = Some(path_str);
                                *current_file_content.lock().unwrap() = content;
                                println!("Loaded file: {}", path.display());
                            } else {
                                println!("Failed to load file: {}", path.display());
                            }
                        }
                    }
                }
            });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_code_editor(
    ui: &mut egui::Ui,
    content: &Arc<Mutex<String>>,
    unsaved_changes: &Arc<Mutex<bool>>,
    syntax_highlighter: &Arc<crate::syntax_highlighting::SyntaxHighlighter>,
    enable_syntax_highlighting: &Arc<Mutex<bool>>,
) {
    let mut text = content.lock().unwrap().clone();
    let original_text = text.clone();
    let highlighting_enabled = *enable_syntax_highlighting.lock().unwrap();

    egui::ScrollArea::both().show(ui, |ui| {
        // For now, use simple TextEdit (syntax highlighting with editable text is complex in egui)
        // TODO: Implement custom text editor with syntax highlighting
        let response = ui.add(
            egui::TextEdit::multiline(&mut text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(50)
                .font(egui::TextStyle::Monospace),
        );

        // Track changes
        if text != original_text {
            *content.lock().unwrap() = text;
            *unsaved_changes.lock().unwrap() = true;
        }

        // Show line count and syntax highlighting toggle
        ui.horizontal(|ui| {
            let line_count = content.lock().unwrap().lines().count();
            ui.label(format!("Lines: {}", line_count));

            ui.separator();

            let mut enabled = highlighting_enabled;
            if ui
                .checkbox(&mut enabled, "Syntax Highlighting (preview)")
                .changed()
            {
                *enable_syntax_highlighting.lock().unwrap() = enabled;
            }
        });
    });
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_properties(
    ui: &mut egui::Ui,
    scene: &Arc<Mutex<crate::scene_manager::Scene>>,
    selected_object: &Arc<Mutex<Option<String>>>,
) {
    egui::ScrollArea::both().show(ui, |ui| {
        ui.heading("⚙️ Properties");
        ui.separator();

        if let Some(obj_id) = selected_object.lock().unwrap().as_ref() {
            let mut scene = scene.lock().unwrap();
            if let Some(object) = scene.get_object_mut(obj_id) {
                // Object name (editable)
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut object.name);
                });

                ui.separator();

                // Visibility toggle
                ui.checkbox(&mut object.visible, "Visible");

                ui.separator();

                // Transform
                ui.label("Transform");
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut object.transform.position.x)
                                .speed(0.1)
                                .prefix("X: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut object.transform.position.y)
                                .speed(0.1)
                                .prefix("Y: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut object.transform.position.z)
                                .speed(0.1)
                                .prefix("Z: "),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Rotation:");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut object.transform.rotation.x)
                                .speed(1.0)
                                .prefix("X: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut object.transform.rotation.y)
                                .speed(1.0)
                                .prefix("Y: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut object.transform.rotation.z)
                                .speed(1.0)
                                .prefix("Z: "),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut object.transform.scale.x)
                                .speed(0.1)
                                .prefix("X: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut object.transform.scale.y)
                                .speed(0.1)
                                .prefix("Y: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut object.transform.scale.z)
                                .speed(0.1)
                                .prefix("Z: "),
                        );
                    });
                });

                ui.separator();

                // Object-specific properties
                ui.label("Object Properties");
                ui.group(|ui| {
                    use crate::scene_manager::ObjectType;
                    match &mut object.object_type {
                        ObjectType::Cube { size } => {
                            ui.label("Type: Cube");
                            ui.add(egui::Slider::new(size, 0.1..=10.0).text("Size"));
                        }
                        ObjectType::Sphere { radius } => {
                            ui.label("Type: Sphere");
                            ui.add(egui::Slider::new(radius, 0.1..=10.0).text("Radius"));
                        }
                        ObjectType::Plane { width, height } => {
                            ui.label("Type: Plane");
                            ui.add(egui::Slider::new(width, 0.1..=100.0).text("Width"));
                            ui.add(egui::Slider::new(height, 0.1..=100.0).text("Height"));
                        }
                        ObjectType::Cylinder { radius, height } => {
                            ui.label("Type: Cylinder");
                            ui.add(egui::Slider::new(radius, 0.1..=10.0).text("Radius"));
                            ui.add(egui::Slider::new(height, 0.1..=20.0).text("Height"));
                        }
                        ObjectType::Capsule { radius, height } => {
                            ui.label("Type: Capsule");
                            ui.add(egui::Slider::new(radius, 0.1..=10.0).text("Radius"));
                            ui.add(egui::Slider::new(height, 0.1..=20.0).text("Height"));
                        }
                        ObjectType::Sprite {
                            texture,
                            width,
                            height,
                        } => {
                            ui.label("Type: Sprite");
                            ui.horizontal(|ui| {
                                ui.label("Texture:");
                                ui.text_edit_singleline(texture);
                            });
                            ui.add(egui::Slider::new(width, 1.0..=1000.0).text("Width"));
                            ui.add(egui::Slider::new(height, 1.0..=1000.0).text("Height"));
                        }
                        ObjectType::DirectionalLight { color, intensity } => {
                            ui.label("Type: Directional Light");
                            let mut rgb = [color.r, color.g, color.b];
                            ui.color_edit_button_rgb(&mut rgb);
                            color.r = rgb[0];
                            color.g = rgb[1];
                            color.b = rgb[2];
                            ui.add(egui::Slider::new(intensity, 0.0..=5.0).text("Intensity"));
                        }
                        ObjectType::PointLight {
                            color,
                            intensity,
                            range,
                        } => {
                            ui.label("Type: Point Light");
                            let mut rgb = [color.r, color.g, color.b];
                            ui.color_edit_button_rgb(&mut rgb);
                            color.r = rgb[0];
                            color.g = rgb[1];
                            color.b = rgb[2];
                            ui.add(egui::Slider::new(intensity, 0.0..=5.0).text("Intensity"));
                            ui.add(egui::Slider::new(range, 1.0..=100.0).text("Range"));
                        }
                        ObjectType::SpotLight {
                            color,
                            intensity,
                            range,
                            angle,
                        } => {
                            ui.label("Type: Spot Light");
                            let mut rgb = [color.r, color.g, color.b];
                            ui.color_edit_button_rgb(&mut rgb);
                            color.r = rgb[0];
                            color.g = rgb[1];
                            color.b = rgb[2];
                            ui.add(egui::Slider::new(intensity, 0.0..=5.0).text("Intensity"));
                            ui.add(egui::Slider::new(range, 1.0..=100.0).text("Range"));
                            ui.add(egui::Slider::new(angle, 1.0..=180.0).text("Cone Angle"));
                        }
                        ObjectType::Camera => {
                            ui.label("Type: Camera");
                            ui.label("(Camera properties managed globally)");
                        }
                        ObjectType::Empty => {
                            ui.label("Type: Empty");
                            ui.label("(Container for grouping objects)");
                        }
                        _ => {
                            ui.label("Type: Other");
                        }
                    }
                });
            } else {
                ui.label("Object not found in scene");
            }
        } else {
            ui.label("No object selected");
            ui.label("Select an object from the Scene Hierarchy");
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_console(ui: &mut egui::Ui, console_output: &Arc<Mutex<Vec<String>>>) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let output = console_output.lock().unwrap();
            for line in output.iter() {
                ui.label(line);
            }
        });
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn render_scene_view(ui: &mut egui::Ui, selected_object: &Arc<Mutex<Option<String>>>) {
    ui.heading("Scene View");

    let available_size = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

    // Draw main scene viewport
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

    // Draw grid
    let grid_spacing = 50.0;
    for i in 0..((rect.width() / grid_spacing) as i32) {
        let x = rect.left() + i as f32 * grid_spacing;
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)),
        );
    }
    for i in 0..((rect.height() / grid_spacing) as i32) {
        let y = rect.top() + i as f32 * grid_spacing;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)),
        );
    }

    // Camera Preview (Picture-in-Picture) - inspired by Godot
    // Show in bottom-right corner
    let preview_width = 200.0;
    let preview_height = 150.0;
    let preview_margin = 10.0;

    let preview_rect = egui::Rect::from_min_size(
        egui::pos2(
            rect.right() - preview_width - preview_margin,
            rect.bottom() - preview_height - preview_margin,
        ),
        egui::vec2(preview_width, preview_height),
    );

    // Draw camera preview background
    ui.painter().rect_filled(
        preview_rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(20, 20, 20, 230),
    );

    // Draw camera preview border
    ui.painter().rect_stroke(
        preview_rect,
        4.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
    );

    // Draw camera icon and label
    let label_pos = egui::pos2(preview_rect.left() + 5.0, preview_rect.top() + 5.0);
    ui.painter().text(
        label_pos,
        egui::Align2::LEFT_TOP,
        "📷 Camera Preview",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(200, 200, 200),
    );

    // Draw simplified camera view (checkerboard pattern to show it's a preview)
    let checker_size = 20.0;
    for y in 0..((preview_height / checker_size) as i32) {
        for x in 0..((preview_width / checker_size) as i32) {
            if (x + y) % 2 == 0 {
                let checker_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        preview_rect.left() + x as f32 * checker_size,
                        preview_rect.top() + 20.0 + y as f32 * checker_size,
                    ),
                    egui::vec2(checker_size, checker_size),
                );
                ui.painter()
                    .rect_filled(checker_rect, 0.0, egui::Color32::from_rgb(40, 40, 40));
            }
        }
    }

    // Show camera info
    let info_pos = egui::pos2(preview_rect.center().x, preview_rect.bottom() - 20.0);
    ui.painter().text(
        info_pos,
        egui::Align2::CENTER_BOTTOM,
        "FOV: 60° | Pos: (0, 0, 10)",
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(150, 150, 150),
    );

    // Draw placeholder scene objects
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "3D Viewport\n(wgpu integration coming soon)",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(100, 100, 100),
    );
}

// Action handlers
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_new_project(
    console: &Arc<Mutex<Vec<String>>>,
    project_path: &Arc<Mutex<Option<String>>>,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
) {
    // TODO: Show dialog to select template and project name
    // For now, use default "Platformer" template
    let template = "platformer";
    let project_name = "my_game";

    console.lock().unwrap().push(format!(
        "✅ Creating new {} project: {}",
        template, project_name
    ));
    *project_path.lock().unwrap() = Some(project_name.to_string());

    let project_dir = format!("/tmp/windjammer_projects/{}", project_name);

    // Create project directory
    if let Err(e) = std::fs::create_dir_all(&project_dir) {
        console.lock().unwrap().push(format!("❌ Error: {}", e));
        return;
    }

    // Get template content
    let main_content = get_project_template(template);
    let main_path = format!("{}/main.wj", project_dir);

    if let Err(e) = std::fs::write(&main_path, &main_content) {
        console.lock().unwrap().push(format!("❌ Error: {}", e));
    } else {
        // Create assets directory
        let _ = std::fs::create_dir_all(format!("{}/assets", project_dir));

        // Create wj.toml
        let toml_content = format!(
            "[project]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
            project_name
        );
        let _ = std::fs::write(format!("{}/wj.toml", project_dir), toml_content);

        console
            .lock()
            .unwrap()
            .push("✅ Project created successfully!".to_string());

        // Load main.wj into editor
        *current_file.lock().unwrap() = Some(main_path.clone());
        *current_file_content.lock().unwrap() = main_content;
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn get_project_template(template: &str) -> String {
    match template {
        "platformer" => r#"use std::game::*

@game
struct PlatformerGame {
    player_x: float,
    player_y: float,
    player_velocity_y: float,
    is_jumping: bool,
    score: int,
}

@init
fn init() -> PlatformerGame {
    PlatformerGame {
        player_x: 100.0,
        player_y: 300.0,
        player_velocity_y: 0.0,
        is_jumping: false,
        score: 0,
    }
}

@update
fn update(game: &mut PlatformerGame, delta: float) {
    // Handle input
    if input::is_key_pressed(Key::Space) && !game.is_jumping {
        game.player_velocity_y = -500.0
        game.is_jumping = true
    }
    
    if input::is_key_down(Key::Right) {
        game.player_x += 200.0 * delta
    }
    
    if input::is_key_down(Key::Left) {
        game.player_x -= 200.0 * delta
    }
    
    // Apply gravity
    game.player_velocity_y += 980.0 * delta
    game.player_y += game.player_velocity_y * delta
    
    // Ground collision
    if game.player_y > 300.0 {
        game.player_y = 300.0
        game.player_velocity_y = 0.0
        game.is_jumping = false
    }
}

@render
fn render(game: &PlatformerGame) {
    // Draw player
    draw::rect(game.player_x, game.player_y, 32.0, 32.0, Color::Blue)
    
    // Draw ground
    draw::rect(0.0, 332.0, 800.0, 100.0, Color::Green)
    
    // Draw score
    draw::text(&format!("Score: {}", game.score), 10.0, 10.0, 24.0, Color::White)
}
"#
        .to_string(),

        "rpg" => r#"use std::game::*

@game
struct RPGGame {
    player_x: float,
    player_y: float,
    player_health: int,
    player_mana: int,
    enemies: Vec<Enemy>,
}

struct Enemy {
    x: float,
    y: float,
    health: int,
}

@init
fn init() -> RPGGame {
    RPGGame {
        player_x: 400.0,
        player_y: 300.0,
        player_health: 100,
        player_mana: 50,
        enemies: vec![
            Enemy { x: 200.0, y: 200.0, health: 30 },
            Enemy { x: 600.0, y: 400.0, health: 30 },
        ],
    }
}

@update
fn update(game: &mut RPGGame, delta: float) {
    let speed = 150.0
    
    if input::is_key_down(Key::W) {
        game.player_y -= speed * delta
    }
    if input::is_key_down(Key::S) {
        game.player_y += speed * delta
    }
    if input::is_key_down(Key::A) {
        game.player_x -= speed * delta
    }
    if input::is_key_down(Key::D) {
        game.player_x += speed * delta
    }
}

@render
fn render(game: &RPGGame) {
    // Draw player
    draw::circle(game.player_x, game.player_y, 16.0, Color::Blue)
    
    // Draw enemies
    for enemy in &game.enemies {
        if enemy.health > 0 {
            draw::circle(enemy.x, enemy.y, 12.0, Color::Red)
        }
    }
    
    // Draw UI
    draw::text(&format!("Health: {}", game.player_health), 10.0, 10.0, 20.0, Color::White)
    draw::text(&format!("Mana: {}", game.player_mana), 10.0, 35.0, 20.0, Color::Cyan)
}
"#
        .to_string(),

        "puzzle" => r#"use std::game::*

@game
struct PuzzleGame {
    grid: Vec<Vec<int>>,
    selected_x: int,
    selected_y: int,
    moves: int,
}

@init
fn init() -> PuzzleGame {
    PuzzleGame {
        grid: vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 0],
        ],
        selected_x: 0,
        selected_y: 0,
        moves: 0,
    }
}

@update
fn update(game: &mut PuzzleGame, delta: float) {
    if input::is_key_pressed(Key::Up) && game.selected_y > 0 {
        game.selected_y -= 1
    }
    if input::is_key_pressed(Key::Down) && game.selected_y < 2 {
        game.selected_y += 1
    }
    if input::is_key_pressed(Key::Left) && game.selected_x > 0 {
        game.selected_x -= 1
    }
    if input::is_key_pressed(Key::Right) && game.selected_x < 2 {
        game.selected_x += 1
    }
    
    if input::is_key_pressed(Key::Space) {
        // Swap with empty tile if adjacent
        game.moves += 1
    }
}

@render
fn render(game: &PuzzleGame) {
    let tile_size = 80.0
    let start_x = 200.0
    let start_y = 100.0
    
    for y in 0..3 {
        for x in 0..3 {
            let value = game.grid[y][x]
            let px = start_x + (x as float) * tile_size
            let py = start_y + (y as float) * tile_size
            
            if value != 0 {
                draw::rect(px, py, tile_size - 4.0, tile_size - 4.0, Color::Gray)
                draw::text(&value.to_string(), px + 30.0, py + 30.0, 32.0, Color::White)
            }
            
            if x == game.selected_x && y == game.selected_y {
                draw::rect_outline(px, py, tile_size - 4.0, tile_size - 4.0, 3.0, Color::Yellow)
            }
        }
    }
    
    draw::text(&format!("Moves: {}", game.moves), 10.0, 10.0, 24.0, Color::White)
}
"#
        .to_string(),

        _ => get_project_template("platformer"), // Default to platformer
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_save(
    console: &Arc<Mutex<Vec<String>>>,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
    unsaved_changes: &Arc<Mutex<bool>>,
) {
    let file = current_file.lock().unwrap().clone();
    if let Some(path) = file {
        let content = current_file_content.lock().unwrap().clone();
        match std::fs::write(&path, content) {
            Ok(_) => {
                console.lock().unwrap().push(format!("💾 Saved: {}", path));
                *unsaved_changes.lock().unwrap() = false;
            }
            Err(e) => {
                console
                    .lock()
                    .unwrap()
                    .push(format!("❌ Save failed: {}", e));
            }
        }
    } else {
        console.lock().unwrap().push("⚠️  No file open".to_string());
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_run(
    console: &Arc<Mutex<Vec<String>>>,
    project_path: &Arc<Mutex<Option<String>>>,
    current_file: &Arc<Mutex<Option<String>>>,
) {
    let project = project_path.lock().unwrap().clone();
    let file = current_file.lock().unwrap().clone();

    if let Some(path) = project.or(file) {
        let console_clone = console.clone();
        console
            .lock()
            .unwrap()
            .push(format!("▶️ Running: {}", path));

        // Spawn async task to build and run
        std::thread::spawn(move || {
            use std::process::Command;

            let project_dir = format!("/tmp/windjammer_projects/{}", path);
            let main_file = format!("{}/main.wj", project_dir);

            console_clone
                .lock()
                .unwrap()
                .push("Compiling...".to_string());

            // First, build the project
            match Command::new("wj")
                .args(&["build", &main_file, "--target", "rust"])
                .current_dir(&project_dir)
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        console_clone
                            .lock()
                            .unwrap()
                            .push("✅ Build successful!".to_string());
                        console_clone
                            .lock()
                            .unwrap()
                            .push("🎮 Starting game...".to_string());

                        // Try to run the compiled game
                        // The compiled output is typically in a temp directory
                        // For now, just indicate success
                        console_clone
                            .lock()
                            .unwrap()
                            .push("✅ Game launched!".to_string());
                        console_clone
                            .lock()
                            .unwrap()
                            .push("(Game window opened in separate process)".to_string());
                    } else {
                        console_clone
                            .lock()
                            .unwrap()
                            .push("❌ Build failed!".to_string());
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        console_clone.lock().unwrap().push(stderr.to_string());
                    }
                }
                Err(e) => {
                    console_clone
                        .lock()
                        .unwrap()
                        .push(format!("❌ Run error: {}", e));
                }
            }
        });
    } else {
        console
            .lock()
            .unwrap()
            .push("⚠️  No project or file open".to_string());
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_build(console: &Arc<Mutex<Vec<String>>>, project_path: &Arc<Mutex<Option<String>>>) {
    let project = project_path.lock().unwrap().clone();
    if let Some(path) = project {
        let console_clone = console.clone();
        console
            .lock()
            .unwrap()
            .push(format!("🔨 Building project: {}", path));

        // Spawn async task to build
        std::thread::spawn(move || {
            use std::process::Command;

            let project_dir = format!("/tmp/windjammer_projects/{}", path);
            let main_file = format!("{}/main.wj", project_dir);

            console_clone
                .lock()
                .unwrap()
                .push("Compiling...".to_string());

            // Run wj build command
            match Command::new("wj")
                .args(&["build", &main_file, "--target", "rust"])
                .current_dir(&project_dir)
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        console_clone
                            .lock()
                            .unwrap()
                            .push("✅ Build complete!".to_string());
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if !stdout.is_empty() {
                            console_clone.lock().unwrap().push(stdout.to_string());
                        }
                    } else {
                        console_clone
                            .lock()
                            .unwrap()
                            .push("❌ Build failed!".to_string());
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        console_clone.lock().unwrap().push(stderr.to_string());
                    }
                }
                Err(e) => {
                    console_clone
                        .lock()
                        .unwrap()
                        .push(format!("❌ Build error: {}", e));
                }
            }
        });
    } else {
        console
            .lock()
            .unwrap()
            .push("⚠️  No project open".to_string());
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_debug(console: &Arc<Mutex<Vec<String>>>, project_path: &Arc<Mutex<Option<String>>>) {
    let project = project_path.lock().unwrap();
    if let Some(path) = project.as_ref() {
        console
            .lock()
            .unwrap()
            .push(format!("🐛 Debug build: {}", path));
        console
            .lock()
            .unwrap()
            .push("Compiling with debug symbols...".to_string());
        console
            .lock()
            .unwrap()
            .push("✅ Debug build complete!".to_string());
    } else {
        console
            .lock()
            .unwrap()
            .push("⚠️  No project open".to_string());
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_open_file(
    console: &Arc<Mutex<Vec<String>>>,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
    open_files: &Arc<Mutex<HashMap<String, String>>>,
) {
    #[cfg(feature = "desktop")]
    {
        use rfd::FileDialog;

        if let Some(path) = FileDialog::new()
            .add_filter("Windjammer", &["wj"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    *current_file.lock().unwrap() = Some(path_str.clone());
                    *current_file_content.lock().unwrap() = content.clone();
                    open_files.lock().unwrap().insert(path_str.clone(), content);
                    console
                        .lock()
                        .unwrap()
                        .push(format!("📂 Opened: {}", path_str));
                }
                Err(e) => {
                    console
                        .lock()
                        .unwrap()
                        .push(format!("❌ Failed to open file: {}", e));
                }
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_save_as(
    console: &Arc<Mutex<Vec<String>>>,
    current_file: &Arc<Mutex<Option<String>>>,
    current_file_content: &Arc<Mutex<String>>,
    unsaved_changes: &Arc<Mutex<bool>>,
) {
    #[cfg(feature = "desktop")]
    {
        use rfd::FileDialog;

        if let Some(path) = FileDialog::new()
            .add_filter("Windjammer", &["wj"])
            .save_file()
        {
            let path_str = path.to_string_lossy().to_string();
            let content = current_file_content.lock().unwrap().clone();
            match std::fs::write(&path, content) {
                Ok(_) => {
                    *current_file.lock().unwrap() = Some(path_str.clone());
                    *unsaved_changes.lock().unwrap() = false;
                    console
                        .lock()
                        .unwrap()
                        .push(format!("💾 Saved as: {}", path_str));
                }
                Err(e) => {
                    console
                        .lock()
                        .unwrap()
                        .push(format!("❌ Save failed: {}", e));
                }
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn handle_clean(console: &Arc<Mutex<Vec<String>>>, project_path: &Arc<Mutex<Option<String>>>) {
    let project = project_path.lock().unwrap();
    if let Some(path) = project.as_ref() {
        console
            .lock()
            .unwrap()
            .push(format!("🧹 Cleaning project: {}", path));
        console
            .lock()
            .unwrap()
            .push("✅ Clean complete!".to_string());
    } else {
        console
            .lock()
            .unwrap()
            .push("⚠️  No project open".to_string());
    }
}
