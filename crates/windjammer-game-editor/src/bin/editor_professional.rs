// Professional Windjammer Game Editor with AAA Framework Panels
// Competitive with Unity, Unreal, and Godot

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console on Windows in release

use windjammer_game_editor::GameEditorPanels;
use windjammer_ui::prelude::*;
use std::sync::{Arc, Mutex};

fn main() {
    println!("🎮 Starting Professional Windjammer Editor");
    println!("📦 Loading AAA Game Framework Panels...");

    // Create game framework panels
    let game_panels = Arc::new(Mutex::new(GameEditorPanels::new()));
    
    // Create the base editor
    let app = EditorApp::new("Windjammer Game Editor".to_string());
    
    println!("✅ Editor ready with full panel suite!");
    
    // Run with game panels integrated
    run_editor_with_game_panels(app, game_panels);
}

fn run_editor_with_game_panels(
    app: EditorApp,
    _game_panels: Arc<Mutex<GameEditorPanels>>,
) {
    // Game panels are now integrated directly into EditorApp (app_docking_v2.rs)
    // They appear in the View menu and render as floating windows
    
    println!("✅ Running editor with integrated game framework panels");
    println!("    Open View menu to access game framework panels!");
    
    app.run();
}
