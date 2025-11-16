// Game Host Editor
// Production-grade game editor with AAA framework integration

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use windjammer_ui::prelude::*;

fn main() {
    println!("🎮 Starting Game Host Editor");
    
    let app = EditorApp::new("Game Host Editor".to_string());
    
    println!("✅ Editor ready!");
    println!("    • Full docking system with file tree, code editor, properties, console");
    println!("    • Game framework panels in View menu");
    println!("    • All panels are dockable and can be rearranged");
    
    app.run();
}

