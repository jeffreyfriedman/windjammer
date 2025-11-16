// Windjammer Game Editor
// Production-grade game editor with AAA framework integration

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use windjammer_ui::prelude::*;

fn main() {
    println!("🎮 Starting Windjammer Game Editor");
    println!("✅ Editor ready!");
    println!("    • Core: File tree, code editor, properties, console, scene view");
    println!("    • Game Framework: 11 panels available via View menu");
    println!("    • All panels are dockable and fully functional");
    
    let app = EditorApp::new("Windjammer Game Editor".to_string());
    app.run();
}
