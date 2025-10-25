use windjammer_game::math::Vec3;
use windjammer_game::physics::PhysicsWorld;

fn main() {
    println!("=== Windjammer Physics Test ===\n");

    let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    println!("Physics world created with gravity: (0, -9.81, 0)");
    println!("Stepping physics simulation...");

    for i in 0..5 {
        world.step();
        println!("  Step {}: Physics simulation running", i + 1);
    }

    println!("\n✅ Physics test complete!");
    println!("✅ Rigid bodies: {}", world.rigid_body_set.len());
    println!("✅ Colliders: {}", world.collider_set.len());
}
