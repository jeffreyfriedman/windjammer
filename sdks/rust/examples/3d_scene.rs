//! 3D Scene Example
//!
//! Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
//!
//! Run with: cargo run --example 3d_scene --features 3d

#[cfg(feature = "3d")]
use windjammer_sdk::prelude::*;

#[cfg(feature = "3d")]
fn main() {
    let mut app = utils::new_3d_app();
    app.add_startup_system(setup);
    app.add_system(update);
    // app.run();
}

#[cfg(feature = "3d")]
fn setup() {
    // Camera
    Camera3D {
        position: Vec3::new(0.0, 5.0, 10.0),
        look_at: Vec3::ZERO,
        fov: 60.0,
    };
    
    // Lights
    PointLight::new(Vec3::new(5.0, 5.0, 5.0), Color::new(1.0, 0.8, 0.6, 1.0), 2000.0);
    PointLight::new(Vec3::new(-5.0, 5.0, 5.0), Color::new(0.6, 0.8, 1.0, 1.0), 1500.0);
    PointLight::new(Vec3::new(0.0, 10.0, -5.0), Color::WHITE, 1000.0);
    
    // Meshes with PBR materials
    Mesh::cube(1.0).with_material(Material {
        albedo: Color::new(0.8, 0.2, 0.2, 1.0),
        metallic: 0.8,
        roughness: 0.2,
        emissive: Color::new(0.5, 0.1, 0.1, 1.0),
    });
    
    Mesh::sphere(1.0, 32).with_material(Material {
        albedo: Color::new(0.2, 0.2, 0.8, 1.0),
        metallic: 0.5,
        roughness: 0.5,
        emissive: Color::new(0.1, 0.1, 0.5, 1.0),
    });
    
    Mesh::plane(10.0).with_material(Material {
        albedo: Color::new(0.3, 0.3, 0.3, 1.0),
        metallic: 0.0,
        roughness: 0.9,
        emissive: Color::BLACK,
    });
    
    // Post-processing
    let mut post = PostProcessing::new();
    post.enable_hdr(true);
    post.set_bloom(BloomSettings { threshold: 1.0, intensity: 0.8, radius: 4.0, soft_knee: 0.5 });
    post.set_ssao(SSAOSettings { radius: 0.5, intensity: 1.5, bias: 0.025, samples: 16 });
    post.set_tone_mapping(ToneMappingMode::ACES, 1.2);
    post.set_color_grading(ColorGrading { temperature: 0.1, tint: 0.0, saturation: 1.2, contrast: 1.1 });
}

#[cfg(feature = "3d")]
fn update() {
    // Rotate objects for dynamic lighting
}

#[cfg(not(feature = "3d"))]
fn main() {
    eprintln!("This example requires the '3d' feature.");
    eprintln!("Run with: cargo run --example 3d_scene --features 3d");
    std::process::exit(1);
}

