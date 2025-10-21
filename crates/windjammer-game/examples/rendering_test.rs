use windjammer_game::math::{Vec3, Vec4};
use windjammer_game::rendering::{Camera, RenderContext};

fn main() {
    println!("=== Windjammer Rendering Test ===\n");

    let camera = Camera::new();
    println!(
        "✅ Camera created at position: ({}, {}, {})",
        camera.position.x, camera.position.y, camera.position.z
    );

    let render_ctx = RenderContext::new();
    println!("✅ Render context initialized");

    let clear_color = Vec4::new(0.1, 0.2, 0.3, 1.0);
    println!(
        "✅ Clear color set: ({}, {}, {}, {})",
        clear_color.x, clear_color.y, clear_color.z, clear_color.w
    );

    println!("\n✅ Rendering system ready!");
}
