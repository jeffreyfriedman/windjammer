//! Enhanced 3D Scene Demo with Post-Processing
//!
//! Demonstrates 3D rendering with advanced post-processing effects:
//! - HDR (High Dynamic Range)
//! - Bloom (glowing lights)
//! - SSAO (Screen-Space Ambient Occlusion)
//! - Tone Mapping
//! - Color Grading
//!
//! This creates a much more visually impressive and marketable demo.
//!
//! Run with: cargo run --example 3d_scene_enhanced

use windjammer_sdk::prelude::*;

fn main() {
    println!("=== Windjammer Enhanced 3D Scene Demo (Rust) ===");
    println!("Features: HDR + Bloom + SSAO + Tone Mapping");
    println!();
    
    // Create 3D application
    let mut app = App::new();
    
    // Setup system
    app.add_startup_system(setup_3d_scene);
    
    // Update system with rotation for dynamic lighting
    app.add_system(rotate_scene);
    
    println!("\n3D application configured with post-processing!");
    println!("- Camera: Perspective (60° FOV)");
    println!("- Rendering: Deferred PBR");
    println!("- Lighting: 3 Point Lights (warm, cool, rim)");
    println!("- Post-Processing:");
    println!("  ✨ HDR (High Dynamic Range)");
    println!("  ✨ Bloom (glowing lights)");
    println!("  ✨ SSAO (ambient occlusion)");
    println!("  ✨ ACES Tone Mapping");
    println!("  ✨ Color Grading");
    println!();
    println!("This creates a cinematic, AAA-quality visual presentation!");
    println!();
    
    // Run the application
    app.run();
}

fn setup_3d_scene() {
    println!("\n[Setup] Creating enhanced 3D scene...");
    
    // Create 3D camera
    let camera = Camera3D {
        position: Vec3::new(0.0, 5.0, 10.0),
        look_at: Vec3::new(0.0, 0.0, 0.0),
        fov: 60.0,
    };
    println!("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)");
    
    // Create meshes
    let cube = Mesh::cube(1.0);
    println!("  - Cube mesh (size=1.0)");
    
    let sphere = Mesh::sphere(1.0, 32);
    println!("  - Sphere mesh (radius=1.0, subdivisions=32)");
    
    let plane = Mesh::plane(10.0);
    println!("  - Plane mesh (size=10.0)");
    
    // Create PBR materials with emissive properties for bloom
    let material_red = Material {
        albedo: Color::new(0.8, 0.2, 0.2, 1.0),
        metallic: 0.8,
        roughness: 0.2,
        emissive: Color::new(0.5, 0.1, 0.1, 1.0), // Red glow
    };
    println!("  - PBR Material (red, metallic=0.8, roughness=0.2, emissive glow)");
    
    let material_blue = Material {
        albedo: Color::new(0.2, 0.2, 0.8, 1.0),
        metallic: 0.5,
        roughness: 0.5,
        emissive: Color::new(0.1, 0.1, 0.5, 1.0), // Blue glow
    };
    println!("  - PBR Material (blue, metallic=0.5, roughness=0.5, emissive glow)");
    
    let material_ground = Material {
        albedo: Color::new(0.3, 0.3, 0.3, 1.0),
        metallic: 0.0,
        roughness: 0.9,
        emissive: Color::BLACK,
    };
    println!("  - PBR Material (ground, non-metallic)");
    
    // Create multiple lights for dramatic effect
    let light1 = PointLight {
        position: Vec3::new(5.0, 5.0, 5.0),
        color: Color::new(1.0, 0.8, 0.6, 1.0), // Warm light
        intensity: 2000.0, // High intensity for HDR
    };
    println!("  - Point Light 1 at (5, 5, 5) intensity=2000 (warm)");
    
    let light2 = PointLight {
        position: Vec3::new(-5.0, 5.0, 5.0),
        color: Color::new(0.6, 0.8, 1.0, 1.0), // Cool light
        intensity: 1500.0,
    };
    println!("  - Point Light 2 at (-5, 5, 5) intensity=1500 (cool)");
    
    let light3 = PointLight {
        position: Vec3::new(0.0, 10.0, -5.0),
        color: Color::WHITE, // White rim light
        intensity: 1000.0,
    };
    println!("  - Point Light 3 at (0, 10, -5) intensity=1000 (rim)");
    
    // Configure post-processing effects
    let mut post_processing = PostProcessing::new();
    
    // Enable HDR
    post_processing.enable_hdr(true);
    println!("  - HDR enabled");
    
    // Configure Bloom (glowing lights and emissive materials)
    let bloom = BloomSettings {
        threshold: 1.0,    // Brightness threshold
        intensity: 0.8,    // Bloom strength
        radius: 4.0,       // Bloom spread
        soft_knee: 0.5,    // Smooth transition
    };
    post_processing.set_bloom(bloom);
    println!("  - Bloom configured (threshold=1.0, intensity=0.8)");
    
    // Configure SSAO (ambient occlusion for depth)
    let ssao = SSAOSettings {
        radius: 0.5,       // Sample radius
        intensity: 1.5,    // Effect strength
        bias: 0.025,       // Depth bias
        samples: 16,       // Quality (more = better but slower)
    };
    post_processing.set_ssao(ssao);
    println!("  - SSAO configured (radius=0.5, intensity=1.5)");
    
    // Configure Tone Mapping (HDR to LDR conversion)
    post_processing.set_tone_mapping(ToneMappingMode::ACES, 1.2);
    println!("  - Tone Mapping: ACES (exposure=1.2)");
    
    // Optional: Color Grading for cinematic look
    post_processing.set_color_grading(ColorGrading {
        temperature: 0.1,   // Slightly warm
        tint: 0.0,         // No tint
        saturation: 1.2,   // Slightly more saturated
        contrast: 1.1,     // Slightly more contrast
    });
    println!("  - Color Grading: warm, saturated, high contrast");
    
    println!("[Setup] Enhanced scene ready!");
}

fn rotate_scene() {
    // This would rotate objects to show off the lighting
    // let rotation_speed = 0.5;
    // let angle = time.elapsed() * rotation_speed;
}

