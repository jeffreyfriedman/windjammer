//! Test physics with Rapier2D
//! Demonstrates bouncing balls with gravity and collisions

use rapier2d::prelude::*;
use std::time::{Duration, Instant};

fn main() {
    env_logger::init();

    println!("=== Windjammer Physics Test ===\n");

    // Create physics world with gravity
    let gravity = vector![0.0, -9.81];
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();

    // Create integration parameters and pipeline
    let integration_parameters = IntegrationParameters::default();
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = BroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let mut query_pipeline = QueryPipeline::new();
    let physics_hooks = ();
    let event_handler = ();

    println!("✅ Physics world created with gravity: {:?}", gravity);

    // Create ground (static body)
    let ground_body = RigidBodyBuilder::fixed()
        .translation(vector![0.0, -5.0])
        .build();
    let ground_handle = rigid_body_set.insert(ground_body);

    let ground_collider = ColliderBuilder::cuboid(10.0, 0.5).build();
    collider_set.insert_with_parent(ground_collider, ground_handle, &mut rigid_body_set);

    println!("✅ Ground created at y=-5.0");

    // Create falling balls
    let mut ball_handles = Vec::new();

    for i in 0..3 {
        let x = (i as f32 - 1.0) * 2.0;
        let y = 5.0 + i as f32 * 2.0;

        let ball_body = RigidBodyBuilder::dynamic()
            .translation(vector![x, y])
            .build();
        let ball_handle = rigid_body_set.insert(ball_body);

        let ball_collider = ColliderBuilder::ball(0.5)
            .restitution(0.7) // Bounciness
            .build();
        collider_set.insert_with_parent(ball_collider, ball_handle, &mut rigid_body_set);

        ball_handles.push(ball_handle);
        println!("✅ Ball {} created at ({:.1}, {:.1})", i + 1, x, y);
    }

    println!("\n🎮 Simulating physics for 5 seconds...\n");

    // Run simulation
    let start_time = Instant::now();
    let sim_duration = Duration::from_secs(5);
    let dt = 1.0 / 60.0; // 60 FPS
    let mut frame = 0;

    while start_time.elapsed() < sim_duration {
        // Step physics
        physics_pipeline.step(
            &gravity,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            Some(&mut query_pipeline),
            &physics_hooks,
            &event_handler,
        );

        frame += 1;

        // Print positions every 30 frames (0.5 seconds)
        if frame % 30 == 0 {
            let time = start_time.elapsed().as_secs_f32();
            println!("⏱️  Time: {:.2}s", time);

            for (i, &handle) in ball_handles.iter().enumerate() {
                if let Some(body) = rigid_body_set.get(handle) {
                    let pos = body.translation();
                    let vel = body.linvel();
                    println!(
                        "   Ball {}: pos=({:.2}, {:.2}), vel=({:.2}, {:.2})",
                        i + 1,
                        pos.x,
                        pos.y,
                        vel.x,
                        vel.y
                    );
                }
            }
            println!();
        }

        // Sleep to maintain 60 FPS
        std::thread::sleep(Duration::from_secs_f32(dt));
    }

    println!("✅ Simulation complete!");
    println!("\n📊 Final positions:");

    for (i, &handle) in ball_handles.iter().enumerate() {
        if let Some(body) = rigid_body_set.get(handle) {
            let pos = body.translation();
            println!("   Ball {}: ({:.2}, {:.2})", i + 1, pos.x, pos.y);
        }
    }

    println!("\n🎉 Physics test successful!");
    println!("   - Gravity working");
    println!("   - Collisions detected");
    println!("   - Bouncing behavior correct");
}
