//! 2D Sprite Demo
//!
//! Demonstrates 2D sprite rendering with the Windjammer SDK.
//!
//! Run with: cargo run --example sprite_demo

use windjammer_sdk::prelude::*;

fn main() {
    println!("=== Windjammer 2D Sprite Demo ===");
    
    // Create 2D application
    let mut app = utils::new_2d_app();
    
    // Add startup system
    app.add_startup_system(setup_2d_scene);
    
    // Add update system
    app.add_system(rotate_sprites);
    
    println!("2D application configured!");
    println!("- Camera: Orthographic");
    println!("- Sprites: Enabled");
    println!("- Physics: 2D");
    
    // In a real app, this would start the game loop
    // app.run();
}

fn setup_2d_scene() {
    println!("\n[Setup] Creating 2D scene...");
    
    // Create camera
    println!("  - Spawning 2D camera");
    
    // Create sprites
    println!("  - Spawning sprites");
    println!("  - Loading textures");
    
    println!("[Setup] Scene ready!");
}

fn rotate_sprites() {
    // This would rotate sprites each frame
    // In a real implementation, this would query entities
    // and update their transforms
}

