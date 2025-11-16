// Asset Browser Module
// Provides visual asset management with thumbnails, filtering, and drag-drop

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Pos2, Rect, Sense, TextureHandle, Ui, Vec2};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::HashMap;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::path::{Path, PathBuf};

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AssetType {
    Image,
    Model,
    Audio,
    Video,
    Script,
    Material,
    Animation,
    Prefab,
    Scene,
    Font,
    Shader,
    Other,
}

impl AssetType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" => AssetType::Image,
            "obj" | "fbx" | "gltf" | "glb" | "dae" | "blend" => AssetType::Model,
            "mp3" | "wav" | "ogg" | "flac" | "aac" => AssetType::Audio,
            "mp4" | "avi" | "mov" | "webm" => AssetType::Video,
            "wj" | "rs" | "js" | "ts" | "py" => AssetType::Script,
            "mat" => AssetType::Material,
            "anim" => AssetType::Animation,
            "prefab" => AssetType::Prefab,
            "scene" => AssetType::Scene,
            "ttf" | "otf" | "woff" | "woff2" => AssetType::Font,
            "glsl" | "wgsl" | "hlsl" | "vert" | "frag" => AssetType::Shader,
            _ => AssetType::Other,
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            AssetType::Image => "🖼️",
            AssetType::Model => "🎲",
            AssetType::Audio => "🔊",
            AssetType::Video => "🎬",
            AssetType::Script => "📜",
            AssetType::Material => "🎨",
            AssetType::Animation => "🎭",
            AssetType::Prefab => "📦",
            AssetType::Scene => "🌍",
            AssetType::Font => "🔤",
            AssetType::Shader => "✨",
            AssetType::Other => "📄",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            AssetType::Image => Color32::from_rgb(100, 150, 255),
            AssetType::Model => Color32::from_rgb(255, 150, 100),
            AssetType::Audio => Color32::from_rgb(150, 255, 150),
            AssetType::Video => Color32::from_rgb(255, 100, 150),
            AssetType::Script => Color32::from_rgb(200, 200, 100),
            AssetType::Material => Color32::from_rgb(255, 150, 255),
            AssetType::Animation => Color32::from_rgb(150, 255, 255),
            AssetType::Prefab => Color32::from_rgb(255, 200, 100),
            AssetType::Scene => Color32::from_rgb(100, 255, 200),
            AssetType::Font => Color32::from_rgb(200, 150, 255),
            AssetType::Shader => Color32::from_rgb(255, 255, 150),
            AssetType::Other => Color32::from_gray(150),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub path: PathBuf,
    pub name: String,
    pub asset_type: AssetType,
    pub size: u64,
    pub modified: std::time::SystemTime,
}

impl Asset {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        if !path.exists() {
            return None;
        }

        let name = path
            .file_name()?
            .to_string_lossy()
            .to_string();

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let asset_type = AssetType::from_extension(extension);

        let metadata = std::fs::metadata(&path).ok()?;
        let size = metadata.len();
        let modified = metadata.modified().ok()?;

        Some(Asset {
            path,
            name,
            asset_type,
            size,
            modified,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
    Grid,
    List,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SortBy {
    Name,
    Type,
    Size,
    Modified,
}

// ============================================================================
// UI COMPONENT (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AssetBrowser {
    current_path: PathBuf,
    assets: Vec<Asset>,
    selected_asset: Option<usize>,
    view_mode: ViewMode,
    sort_by: SortBy,
    filter_text: String,
    filter_type: Option<AssetType>,
    thumbnail_size: f32,
    dragging_asset: Option<usize>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AssetBrowser {
    pub fn new(root_path: PathBuf) -> Self {
        let mut browser = Self {
            current_path: root_path.clone(),
            assets: Vec::new(),
            selected_asset: None,
            view_mode: ViewMode::Grid,
            sort_by: SortBy::Name,
            filter_text: String::new(),
            filter_type: None,
            thumbnail_size: 80.0,
            dragging_asset: None,
        };
        browser.refresh();
        browser
    }

    pub fn refresh(&mut self) {
        self.assets.clear();
        
        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            for entry in entries.flatten() {
                if let Some(asset) = Asset::from_path(entry.path()) {
                    self.assets.push(asset);
                }
            }
        }

        self.sort_assets();
    }

    fn sort_assets(&mut self) {
        match self.sort_by {
            SortBy::Name => self.assets.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::Type => self.assets.sort_by(|a, b| {
                format!("{:?}", a.asset_type).cmp(&format!("{:?}", b.asset_type))
            }),
            SortBy::Size => self.assets.sort_by(|a, b| a.size.cmp(&b.size)),
            SortBy::Modified => self.assets.sort_by(|a, b| a.modified.cmp(&b.modified)),
        }
    }

    fn filtered_assets(&self) -> Vec<(usize, &Asset)> {
        self.assets
            .iter()
            .enumerate()
            .filter(|(_, asset)| {
                // Filter by type
                if let Some(filter_type) = &self.filter_type {
                    if &asset.asset_type != filter_type {
                        return false;
                    }
                }

                // Filter by text
                if !self.filter_text.is_empty() {
                    if !asset.name.to_lowercase().contains(&self.filter_text.to_lowercase()) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.heading("📁 Asset Browser");
            ui.separator();

            // Navigation
            if ui.button("⬆️ Up").clicked() {
                if let Some(parent) = self.current_path.parent() {
                    self.current_path = parent.to_path_buf();
                    self.refresh();
                }
            }

            if ui.button("🔄 Refresh").clicked() {
                self.refresh();
            }

            ui.separator();

            // View mode
            ui.selectable_value(&mut self.view_mode, ViewMode::Grid, "🔲 Grid");
            ui.selectable_value(&mut self.view_mode, ViewMode::List, "📋 List");

            ui.separator();

            // Sort by
            ui.label("Sort:");
            egui::ComboBox::from_id_salt("sort_by")
                .selected_text(format!("{:?}", self.sort_by))
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut self.sort_by, SortBy::Name, "Name").clicked() {
                        self.sort_assets();
                    }
                    if ui.selectable_value(&mut self.sort_by, SortBy::Type, "Type").clicked() {
                        self.sort_assets();
                    }
                    if ui.selectable_value(&mut self.sort_by, SortBy::Size, "Size").clicked() {
                        self.sort_assets();
                    }
                    if ui.selectable_value(&mut self.sort_by, SortBy::Modified, "Modified").clicked() {
                        self.sort_assets();
                    }
                });
        });

        ui.separator();

        // Filter bar
        ui.horizontal(|ui| {
            ui.label("🔍 Filter:");
            if ui.text_edit_singleline(&mut self.filter_text).changed() {
                // Filter updates automatically
            }

            ui.separator();

            // Type filters
            let asset_types = [
                AssetType::Image,
                AssetType::Model,
                AssetType::Audio,
                AssetType::Script,
                AssetType::Material,
            ];

            for asset_type in asset_types {
                let is_active = self.filter_type == Some(asset_type.clone());
                let button = egui::Button::new(format!("{} {:?}", asset_type.icon(), asset_type))
                    .fill(if is_active { asset_type.color() } else { Color32::from_gray(40) });

                if ui.add(button).clicked() {
                    self.filter_type = if is_active {
                        None
                    } else {
                        Some(asset_type)
                    };
                }
            }

            if self.filter_type.is_some() && ui.button("❌ Clear").clicked() {
                self.filter_type = None;
            }
        });

        ui.separator();

        // Current path
        ui.label(format!("📂 {}", self.current_path.display()));

        ui.separator();

        // Asset view
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Collect filtered assets to avoid borrow checker issues
            let filtered: Vec<(usize, Asset)> = self.filtered_assets()
                .into_iter()
                .map(|(idx, asset)| (idx, asset.clone()))
                .collect();

            if filtered.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No assets found");
                });
                return;
            }

            // Convert to references for rendering
            let filtered_refs: Vec<(usize, &Asset)> = filtered.iter()
                .map(|(idx, asset)| (*idx, asset))
                .collect();

            match self.view_mode {
                ViewMode::Grid => {
                    let thumbnail_size = self.thumbnail_size;
                    Self::render_grid_view_static(ui, &filtered_refs, &mut self.selected_asset, &mut self.dragging_asset, thumbnail_size);
                }
                ViewMode::List => {
                    Self::render_list_view_static(ui, &filtered_refs, &mut self.selected_asset, &mut self.dragging_asset);
                }
            }
        });

        // Status bar
        ui.separator();
        ui.horizontal(|ui| {
            let filtered = self.filtered_assets();
            ui.label(format!("{} assets ({} total)", filtered.len(), self.assets.len()));

            if let Some(idx) = self.selected_asset {
                if let Some(asset) = self.assets.get(idx) {
                    ui.separator();
                    ui.label(format!("Selected: {}", asset.name));
                    ui.label(format!("Size: {} bytes", asset.size));
                }
            }
        });
    }

    fn render_grid_view_static(
        ui: &mut Ui,
        filtered: &[(usize, &Asset)],
        selected_asset: &mut Option<usize>,
        dragging_asset: &mut Option<usize>,
        thumbnail_size: f32,
    ) {
        let available_width = ui.available_width();
        let item_width = thumbnail_size + 20.0;
        let _columns = (available_width / item_width).floor().max(1.0) as usize;

        ui.horizontal_wrapped(|ui| {
            for (idx, asset) in filtered {
                let is_selected = *selected_asset == Some(*idx);

                ui.vertical(|ui| {
                    ui.set_width(item_width);

                    // Thumbnail
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(thumbnail_size, thumbnail_size),
                        Sense::click_and_drag(),
                    );

                    let bg_color = if is_selected {
                        Color32::from_rgb(70, 120, 200)
                    } else if response.hovered() {
                        Color32::from_gray(60)
                    } else {
                        Color32::from_gray(40)
                    };

                    ui.painter().rect_filled(rect, 5.0, bg_color);

                    // Icon
                    let icon_text = format!("{}\n{:?}", asset.asset_type.icon(), asset.asset_type);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon_text,
                        egui::FontId::proportional(20.0),
                        asset.asset_type.color(),
                    );

                    if response.clicked() {
                        *selected_asset = Some(*idx);
                    }

                    if response.double_clicked() {
                        // TODO: Open asset
                        println!("Open asset: {}", asset.path.display());
                    }

                    if response.drag_started() {
                        *dragging_asset = Some(*idx);
                    }

                    // Name
                    ui.label(&asset.name);
                });
            }
        });
    }

    fn render_list_view_static(
        ui: &mut Ui,
        filtered: &[(usize, &Asset)],
        selected_asset: &mut Option<usize>,
        dragging_asset: &mut Option<usize>,
    ) {
        for (idx, asset) in filtered {
            let is_selected = *selected_asset == Some(*idx);

            let response = ui.horizontal(|ui| {
                ui.label(asset.asset_type.icon());
                ui.label(&asset.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} bytes", asset.size));
                    ui.label(format!("{:?}", asset.asset_type));
                });
            });

            let response = response.response.interact(Sense::click_and_drag());

            if response.clicked() {
                *selected_asset = Some(*idx);
            }

            if response.double_clicked() {
                // TODO: Open asset
                println!("Open asset: {}", asset.path.display());
            }

            if response.drag_started() {
                *dragging_asset = Some(*idx);
            }

            if is_selected {
                ui.painter().rect_stroke(
                    response.rect,
                    0.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
                );
            }
        }
    }

    pub fn get_selected_asset(&self) -> Option<&Asset> {
        self.selected_asset.and_then(|idx| self.assets.get(idx))
    }

    pub fn get_dragging_asset(&self) -> Option<&Asset> {
        self.dragging_asset.and_then(|idx| self.assets.get(idx))
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AssetBrowser {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

