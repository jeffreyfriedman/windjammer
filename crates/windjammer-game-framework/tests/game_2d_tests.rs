//! Comprehensive tests for 2D game systems

use windjammer_game_framework::camera2d::Camera2D;
use windjammer_game_framework::ecs::{Entity, World};
use windjammer_game_framework::input::{Input, KeyCode, MouseButton};
use windjammer_game_framework::math::Vec2;
use windjammer_game_framework::rendering::sprite::{Sprite, SpriteBatch};
use windjammer_game_framework::transform::Transform2D;

// ============================================================================
// Camera2D Tests
// ============================================================================

#[test]
fn test_camera2d_creation() {
    let camera = Camera2D::new(800.0, 600.0);
    assert_eq!(camera.position.x, 0.0);
    assert_eq!(camera.position.y, 0.0);
    assert_eq!(camera.zoom, 1.0);
    assert_eq!(camera.rotation, 0.0);
}

#[test]
fn test_camera2d_movement() {
    let mut camera = Camera2D::new(800.0, 600.0);
    camera.translate(Vec2::new(100.0, 50.0));

    assert_eq!(camera.position.x, 100.0);
    assert_eq!(camera.position.y, 50.0);
}

#[test]
fn test_camera2d_zoom() {
    let mut camera = Camera2D::new(800.0, 600.0);
    camera.set_zoom(2.0);

    assert_eq!(camera.zoom, 2.0);
}

#[test]
fn test_camera2d_rotation() {
    let mut camera = Camera2D::new(800.0, 600.0);
    let angle = std::f32::consts::PI / 4.0; // 45 degrees
    camera.rotate(angle);

    assert!((camera.rotation - angle).abs() < 0.001);
}

#[test]
fn test_camera2d_follow() {
    let mut camera = Camera2D::new(800.0, 600.0);
    let target = Vec2::new(100.0, 100.0);

    // Follow with moderate smoothness should move camera closer
    camera.follow(target, 5.0, 0.1);

    assert!(camera.position.x > 0.0);
    assert!(camera.position.y > 0.0);
    // Camera should move towards target but not overshoot
    assert!(camera.position.x <= 100.0);
    assert!(camera.position.y <= 100.0);
}

#[test]
fn test_camera2d_clamp_to_bounds() {
    let mut camera = Camera2D::new(800.0, 600.0);
    camera.translate(Vec2::new(1000.0, 1000.0));

    let min = Vec2::new(0.0, 0.0);
    let max = Vec2::new(800.0, 600.0);
    camera.clamp_to_bounds(min, max);

    assert_eq!(camera.position.x, 800.0);
    assert_eq!(camera.position.y, 600.0);
}

// ============================================================================
// Transform2D Tests
// ============================================================================

#[test]
fn test_transform2d_creation() {
    let transform = Transform2D::new();
    assert_eq!(transform.position.x, 0.0);
    assert_eq!(transform.position.y, 0.0);
    assert_eq!(transform.rotation, 0.0);
    assert_eq!(transform.scale.x, 1.0);
    assert_eq!(transform.scale.y, 1.0);
}

#[test]
fn test_transform2d_translation() {
    let mut transform = Transform2D::new();
    transform.translate(Vec2::new(10.0, 20.0));

    assert_eq!(transform.position.x, 10.0);
    assert_eq!(transform.position.y, 20.0);
}

#[test]
fn test_transform2d_rotation() {
    let mut transform = Transform2D::new();
    let angle = std::f32::consts::PI / 2.0; // 90 degrees
    transform.rotate(angle);

    assert!((transform.rotation - angle).abs() < 0.001);
}

#[test]
fn test_transform2d_scale() {
    let mut transform = Transform2D::new();
    transform.scale_by(Vec2::new(2.0, 3.0));

    assert_eq!(transform.scale.x, 2.0);
    assert_eq!(transform.scale.y, 3.0);
}

// ============================================================================
// Sprite Tests
// ============================================================================

#[test]
fn test_sprite_creation() {
    let sprite = Sprite::new(Vec2::new(100.0, 200.0), Vec2::new(32.0, 32.0));

    assert_eq!(sprite.position.x, 100.0);
    assert_eq!(sprite.size.x, 32.0);
}

#[test]
fn test_sprite_with_color() {
    let sprite =
        Sprite::new(Vec2::new(0.0, 0.0), Vec2::new(32.0, 32.0)).with_color([1.0, 0.0, 0.0, 1.0]);

    assert_eq!(sprite.color, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_sprite_batch_creation() {
    let batch = SpriteBatch::new();
    assert_eq!(batch.sprites().len(), 0);
}

#[test]
fn test_sprite_batch_add() {
    let mut batch = SpriteBatch::new();
    let sprite = Sprite::new(Vec2::new(0.0, 0.0), Vec2::new(32.0, 32.0));

    batch.add(sprite);

    assert_eq!(batch.sprites().len(), 1);
}

#[test]
fn test_sprite_batch_clear() {
    let mut batch = SpriteBatch::new();
    let sprite = Sprite::new(Vec2::new(0.0, 0.0), Vec2::new(32.0, 32.0));

    batch.add(sprite);
    batch.clear();

    assert_eq!(batch.sprites().len(), 0);
}

// ============================================================================
// Input System Tests
// ============================================================================

#[test]
fn test_input_key_press() {
    let mut input = Input::new();

    input.press_key(KeyCode::Space);
    assert!(input.is_key_pressed(KeyCode::Space));
    assert!(input.is_key_just_pressed(KeyCode::Space));

    input.clear_frame_state();
    assert!(input.is_key_pressed(KeyCode::Space));
    assert!(!input.is_key_just_pressed(KeyCode::Space)); // Only true on first frame
}

#[test]
fn test_input_key_release() {
    let mut input = Input::new();

    input.press_key(KeyCode::Space);
    input.clear_frame_state();
    input.release_key(KeyCode::Space);

    assert!(!input.is_key_pressed(KeyCode::Space));
    assert!(input.is_key_just_released(KeyCode::Space));

    input.clear_frame_state();
    assert!(!input.is_key_just_released(KeyCode::Space)); // Only true on first frame
}

#[test]
fn test_input_mouse_button() {
    let mut input = Input::new();

    input.press_mouse_button(MouseButton::Left);
    assert!(input.is_mouse_button_pressed(MouseButton::Left));
    assert!(input.is_mouse_button_just_pressed(MouseButton::Left));

    input.clear_frame_state();
    assert!(input.is_mouse_button_pressed(MouseButton::Left));
    assert!(!input.is_mouse_button_just_pressed(MouseButton::Left));
}

#[test]
fn test_input_mouse_position() {
    let mut input = Input::new();

    input.set_mouse_position(100.0, 200.0);
    let (x, y) = input.mouse_position();

    assert_eq!(x, 100.0);
    assert_eq!(y, 200.0);
}

#[test]
fn test_input_mouse_delta() {
    let mut input = Input::new();

    input.set_mouse_delta(10.0, 20.0);

    let (dx, dy) = input.mouse_delta();
    assert_eq!(dx, 10.0);
    assert_eq!(dy, 20.0);
}

#[test]
fn test_input_mouse_wheel() {
    let mut input = Input::new();

    input.set_mouse_wheel_delta(1.5);
    assert_eq!(input.mouse_wheel_delta(), 1.5);
}

// ============================================================================
// ECS Integration Tests
// ============================================================================

#[derive(Clone, Copy, Debug)]
struct Position2D {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct Velocity2D {
    x: f32,
    y: f32,
}

#[test]
fn test_ecs_spawn_entity() {
    let mut world = World::new();
    let entity = world.spawn().build();

    // Entity should be in the world
    assert!(world.entities().contains(&entity));
}

#[test]
fn test_ecs_add_component() {
    let mut world = World::new();
    let entity = world.spawn().build();

    world.add_component(entity, Position2D { x: 10.0, y: 20.0 });

    let pos = world.get_component::<Position2D>(entity);
    assert!(pos.is_some());
    assert_eq!(pos.unwrap().x, 10.0);
}

#[test]
fn test_ecs_remove_component() {
    let mut world = World::new();
    let entity = world.spawn().build();

    world.add_component(entity, Position2D { x: 10.0, y: 20.0 });
    world.remove_component::<Position2D>(entity);

    let pos = world.get_component::<Position2D>(entity);
    assert!(pos.is_none());
}

#[test]
fn test_ecs_query_entities() {
    let mut world = World::new();

    let e1 = world.spawn().build();
    world.add_component(e1, Position2D { x: 1.0, y: 1.0 });

    let e2 = world.spawn().build();
    world.add_component(e2, Position2D { x: 2.0, y: 2.0 });

    let _e3 = world.spawn().build();
    // No position component

    let results = world.query::<Position2D>();
    assert_eq!(results.len(), 2);

    // Verify we got the right entities
    let entities: Vec<Entity> = results.iter().map(|(e, _)| *e).collect();
    assert!(entities.contains(&e1));
    assert!(entities.contains(&e2));
}

#[test]
fn test_ecs_destroy_entity() {
    let mut world = World::new();
    let entity = world.spawn().build();

    assert!(world.entities().contains(&entity));

    world.despawn(entity);

    assert!(!world.entities().contains(&entity));
}

// ============================================================================
// Game Loop Integration Test
// ============================================================================

#[test]
fn test_simple_game_loop() {
    let mut world = World::new();

    // Create player entity
    let player = world.spawn().build();
    world.add_component(player, Position2D { x: 0.0, y: 0.0 });
    world.add_component(player, Velocity2D { x: 100.0, y: 0.0 });

    // Simulate one frame
    let delta = 0.016; // ~60 FPS

    // Update physics using query2_mut
    let entities = world.query2_mut::<Position2D, Velocity2D>();
    for (_entity, pos, vel) in entities {
        pos.x += vel.x * delta;
        pos.y += vel.y * delta;
    }

    // Check position updated
    let pos = world.get_component::<Position2D>(player).unwrap();
    assert!(pos.x > 0.0);
    assert!((pos.x - 1.6).abs() < 0.01); // 100 * 0.016
}

#[test]
fn test_collision_detection() {
    let mut world = World::new();

    // Create two entities
    let e1 = world.spawn().build();
    world.add_component(e1, Position2D { x: 0.0, y: 0.0 });

    let e2 = world.spawn().build();
    world.add_component(e2, Position2D { x: 5.0, y: 5.0 });

    // Simple distance-based collision
    let pos1 = world.get_component::<Position2D>(e1).unwrap();
    let pos2 = world.get_component::<Position2D>(e2).unwrap();

    let dx = pos2.x - pos1.x;
    let dy = pos2.y - pos1.y;
    let distance = (dx * dx + dy * dy).sqrt();

    let collision_radius = 10.0;
    assert!(distance < collision_radius); // Should collide
}
