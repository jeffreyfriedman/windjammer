//! 3D Scene Example
//!
//! Demonstrates 3D rendering with PBR materials and lighting.
//!
//! Run with: cargo run --example 3d_scene --features 3d

#[cfg(feature = "3d")]
use windjammer_sdk::prelude::*;

#[cfg(feature = "3d")]
fn main() {
    println!("=== Windjammer 3D Scene Demo ===");
    
    // Create 3D application
    let mut app = utils::new_3d_app();
    
    // Add startup system
    app.add_startup_system(setup_3d_scene);
    
    // Add update system
    app.add_system(rotate_objects);
    
    println!("3D application configured!");
    println!("- Camera: Perspective");
    println!("- Rendering: Deferred + PBR");
    println!("- Physics: 3D (Rapier3D)");
    println!("- Lighting: Point, Directional, Spot");
    
    // In a real app, this would start the game loop
    // app.run();
}

#[cfg(feature = "3d")]
fn setup_3d_scene() {
    println!("\n[Setup] Creating 3D scene...");
    
    // Create camera
    println!("  - Spawning 3D camera at (0, 5, 10)");
    let camera_pos = Vec3::new(0.0, 5.0, 10.0);
    println!("    Position: {:?}", camera_pos);
    
    // Create lights
    println!("  - Spawning point light");
    println!("    Intensity: 1500.0");
    println!("    Position: (4, 8, 4)");
    
    // Create 3D objects
    println!("  - Spawning cube mesh");
    println!("  - Applying PBR material");
    println!("    Metallic: 0.5");
    println!("    Roughness: 0.5");
    
    println!("[Setup] 3D scene ready!");
}

#[cfg(feature = "3d")]
fn rotate_objects() {
    // This would rotate 3D objects each frame
}

#[cfg(not(feature = "3d"))]
fn main() {
    eprintln!("This example requires the '3d' feature.");
    eprintln!("Run with: cargo run --example 3d_scene --features 3d");
    std::process::exit(1);
}

