// Performance Profiler Visualization Panel
// Real-time performance monitoring and analysis

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Ui};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::VecDeque;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct ProfilerPanel {
    // Performance metrics
    frame_times: VecDeque<f32>,
    fps_history: VecDeque<f32>,
    memory_usage: VecDeque<f32>,
    
    // Current stats
    current_fps: f32,
    current_frame_time: f32,
    current_memory_mb: f32,
    
    // Settings
    max_history: usize,
    show_frame_times: bool,
    show_fps: bool,
    show_memory: bool,
    #[allow(dead_code)]
    show_detailed_stats: bool,
    
    // Profiling scopes
    profiling_enabled: bool,
    selected_scope: Option<String>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for ProfilerPanel {
    fn default() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(300),
            fps_history: VecDeque::with_capacity(300),
            memory_usage: VecDeque::with_capacity(300),
            current_fps: 60.0,
            current_frame_time: 16.67,
            current_memory_mb: 128.0,
            max_history: 300,
            show_frame_times: true,
            show_fps: true,
            show_memory: true,
            show_detailed_stats: false,
            profiling_enabled: true,
            selected_scope: None,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl ProfilerPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("📊 Performance Profiler");
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Controls
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.profiling_enabled, "Enable Profiling");
                
                if ui.button("🔄 Reset").clicked() {
                    self.frame_times.clear();
                    self.fps_history.clear();
                    self.memory_usage.clear();
                }
                
                if ui.button("📊 Export Data").clicked() {
                    self.export_profiling_data();
                }
            });
            
            ui.add_space(10.0);
            
            // Current Stats
            ui.group(|ui| {
                ui.heading("Current Stats");
                
                ui.horizontal(|ui| {
                    ui.label("FPS:");
                    ui.colored_label(
                        if self.current_fps >= 60.0 {
                            Color32::GREEN
                        } else if self.current_fps >= 30.0 {
                            Color32::YELLOW
                        } else {
                            Color32::RED
                        },
                        format!("{:.1}", self.current_fps),
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("Frame Time:");
                    ui.colored_label(
                        if self.current_frame_time <= 16.67 {
                            Color32::GREEN
                        } else if self.current_frame_time <= 33.33 {
                            Color32::YELLOW
                        } else {
                            Color32::RED
                        },
                        format!("{:.2} ms", self.current_frame_time),
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("Memory:");
                    ui.label(format!("{:.1} MB", self.current_memory_mb));
                });
            });
            
            ui.add_space(10.0);
            
            // Graph Options
            ui.collapsing("Graph Options", |ui| {
                ui.checkbox(&mut self.show_frame_times, "Show Frame Times");
                ui.checkbox(&mut self.show_fps, "Show FPS");
                ui.checkbox(&mut self.show_memory, "Show Memory Usage");
                
                ui.horizontal(|ui| {
                    ui.label("History Size:");
                    ui.add(egui::Slider::new(&mut self.max_history, 60..=600).suffix(" frames"));
                });
            });
            
            ui.add_space(10.0);
            
            // Frame Time Graph
            if self.show_frame_times {
                ui.group(|ui| {
                    ui.heading("Frame Time (ms)");
                    self.render_graph(
                        ui,
                        &self.frame_times,
                        0.0,
                        50.0,
                        Color32::from_rgb(100, 200, 255),
                        "ms",
                    );
                });
                ui.add_space(5.0);
            }
            
            // FPS Graph
            if self.show_fps {
                ui.group(|ui| {
                    ui.heading("FPS");
                    self.render_graph(
                        ui,
                        &self.fps_history,
                        0.0,
                        120.0,
                        Color32::from_rgb(100, 255, 100),
                        "fps",
                    );
                });
                ui.add_space(5.0);
            }
            
            // Memory Graph
            if self.show_memory {
                ui.group(|ui| {
                    ui.heading("Memory Usage (MB)");
                    self.render_graph(
                        ui,
                        &self.memory_usage,
                        0.0,
                        512.0,
                        Color32::from_rgb(255, 200, 100),
                        "MB",
                    );
                });
                ui.add_space(5.0);
            }
            
            // Detailed Stats
            ui.collapsing("Detailed Statistics", |ui| {
                if !self.frame_times.is_empty() {
                    let avg_frame_time: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
                    let min_frame_time = self.frame_times.iter().copied().fold(f32::INFINITY, f32::min);
                    let max_frame_time = self.frame_times.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    
                    ui.label(format!("Average Frame Time: {:.2} ms", avg_frame_time));
                    ui.label(format!("Min Frame Time: {:.2} ms", min_frame_time));
                    ui.label(format!("Max Frame Time: {:.2} ms", max_frame_time));
                    
                    if !self.fps_history.is_empty() {
                        let avg_fps: f32 = self.fps_history.iter().sum::<f32>() / self.fps_history.len() as f32;
                        ui.label(format!("Average FPS: {:.1}", avg_fps));
                    }
                }
            });
            
            ui.add_space(10.0);
            
            // Profiling Scopes
            ui.collapsing("Profiling Scopes", |ui| {
                ui.label("Select a scope to view detailed timing:");
                
                let scopes = vec![
                    "Update Loop",
                    "Render",
                    "Physics",
                    "AI",
                    "Audio",
                    "Input",
                    "Networking",
                ];
                
                for scope in scopes {
                    let is_selected = self.selected_scope.as_deref() == Some(scope);
                    if ui.selectable_label(is_selected, scope).clicked() {
                        self.selected_scope = Some(scope.to_string());
                    }
                }
                
                if let Some(scope) = &self.selected_scope {
                    ui.separator();
                    ui.label(format!("Scope: {}", scope));
                    ui.label("(Detailed timing data would appear here)");
                }
            });
            
            ui.add_space(10.0);
            
            // System Info
            ui.collapsing("System Information", |ui| {
                ui.label("Platform: Desktop");
                ui.label("Renderer: wgpu");
                ui.label("CPU Cores: (detected at runtime)");
                ui.label("GPU: (detected at runtime)");
            });
        });
        
        // Simulate data update (in real implementation, this would come from the game framework)
        if self.profiling_enabled {
            self.update_simulated_data();
        }
    }
    
    fn render_graph(
        &self,
        ui: &mut Ui,
        data: &VecDeque<f32>,
        min_val: f32,
        max_val: f32,
        color: Color32,
        unit: &str,
    ) {
        let available_size = ui.available_size();
        let graph_height = 100.0;
        let graph_size = egui::vec2(available_size.x - 20.0, graph_height);
        
        let (response, painter) = ui.allocate_painter(graph_size, egui::Sense::hover());
        let rect = response.rect;
        
        // Background
        painter.rect_filled(rect, 2.0, Color32::from_rgb(30, 30, 30));
        
        // Grid lines
        for i in 0..5 {
            let y = rect.min.y + (rect.height() * i as f32 / 4.0);
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 50)),
            );
        }
        
        // Data points
        if data.len() >= 2 {
            let points: Vec<egui::Pos2> = data
                .iter()
                .enumerate()
                .map(|(i, &value)| {
                    let x = rect.min.x + (rect.width() * i as f32 / (data.len() - 1) as f32);
                    let normalized = ((value - min_val) / (max_val - min_val)).clamp(0.0, 1.0);
                    let y = rect.max.y - (rect.height() * normalized);
                    egui::pos2(x, y)
                })
                .collect();
            
            // Draw lines
            for window in points.windows(2) {
                painter.line_segment(
                    [window[0], window[1]],
                    egui::Stroke::new(2.0, color),
                );
            }
        }
        
        // Current value label
        if let Some(&last_value) = data.back() {
            ui.label(format!("Current: {:.2} {}", last_value, unit));
        }
    }
    
    fn update_simulated_data(&mut self) {
        // Simulate performance data (in real implementation, this comes from the profiler)
        use std::f32::consts::PI;
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();
        
        // Simulate frame time with some variation
        let base_frame_time = 16.67;
        let variation = (time * 2.0).sin() * 3.0;
        self.current_frame_time = base_frame_time + variation;
        
        // Calculate FPS from frame time
        self.current_fps = 1000.0 / self.current_frame_time;
        
        // Simulate memory usage
        self.current_memory_mb = 128.0 + (time * 0.5).sin() * 32.0;
        
        // Add to history
        self.frame_times.push_back(self.current_frame_time);
        self.fps_history.push_back(self.current_fps);
        self.memory_usage.push_back(self.current_memory_mb);
        
        // Trim history
        while self.frame_times.len() > self.max_history {
            self.frame_times.pop_front();
        }
        while self.fps_history.len() > self.max_history {
            self.fps_history.pop_front();
        }
        while self.memory_usage.len() > self.max_history {
            self.memory_usage.pop_front();
        }
    }
    
    fn export_profiling_data(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .save_file()
        {
            println!("📊 Exporting profiling data to: {}", path.display());
            // TODO: Actually export the data
        }
    }
}
