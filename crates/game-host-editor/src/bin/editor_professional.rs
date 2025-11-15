// Professional Game Host Editor
// Competitive with Unity, Unreal, and Godot

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console on Windows in release

fn main() {
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    {
        println!("🎮 Starting Professional Windjammer Editor");
        
        let app = windjammer_ui::EditorApp::new("Game Host Editor".to_string());
        app.run();
    }
    
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "desktop")))]
    {
        eprintln!("Error: This binary requires the 'desktop' feature to be enabled.");
        eprintln!("Run with: cargo run --bin editor_professional --features desktop");
        std::process::exit(1);
    }
}

