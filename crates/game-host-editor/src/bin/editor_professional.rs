// Professional Game Host Editor
// Competitive with Unity, Unreal, and Godot

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console on Windows in release

use windjammer_ui::prelude::*;

fn main() {
    println!("🎮 Starting Professional Windjammer Editor");
    
    let app = EditorApp::new("Game Host Editor".to_string());
    app.run();
}

