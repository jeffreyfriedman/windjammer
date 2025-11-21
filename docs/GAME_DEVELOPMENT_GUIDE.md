# Windjammer Game Development Guide

## Overview

The Windjammer Game Framework (`windjammer-game`) provides a powerful ECS-based game engine for 2D and 3D games that compile to native or WASM.

## Status

**Current Status**: In Development

The game framework is being actively developed with the following planned features:

### Planned Features

#### Core Systems
- ✅ ECS (Entity-Component-System) architecture
- ✅ Entity management
- ✅ Component queries
- ✅ System scheduling

#### 2D Features
- ✅ 2D rendering with sprites
- ✅ Transform system (position, rotation, scale)
- ✅ 2D camera
- ✅ Input handling (keyboard, mouse)
- ✅ Game loop (update/render cycle)

#### 3D Features
- ✅ 3D rendering with meshes
- ✅ 3D camera (perspective projection)
- ✅ Lighting system (directional, point, spot)
- ✅ Material system

#### Physics
- ✅ Rapier2D integration (2D physics)
- ✅ Rapier3D integration (3D physics)
- ✅ Collision detection
- ✅ Rigid body dynamics

#### Audio
- ✅ Sound effects
- ✅ Background music
- ✅ 3D spatial audio

#### Platform Support
- ✅ Native (Windows, macOS, Linux)
- ✅ Web (WASM)
- ✅ Mobile (iOS, Android) - via Tauri

## Architecture

### ECS Pattern

The framework uses an Entity-Component-System architecture:

```
Entity: Unique ID
    ↓
Components: Data (Position, Sprite, Health)
    ↓
Systems: Logic (Movement, Rendering, Physics)
```

**Benefits**:
- Data-oriented design
- High performance
- Easy to reason about
- Composable gameplay

### Example Structure

```windjammer
@game SimpleGame

// Components
struct Position {
    x: f32,
    y: f32
}

struct Velocity {
    dx: f32,
    dy: f32
}

struct Sprite {
    texture: String,
    width: f32,
    height: f32
}

// Systems
fn movement_system(world: &mut World) {
    for entity in world.query::<(Position, Velocity)>() {
        entity.position.x += entity.velocity.dx
        entity.position.y += entity.velocity.dy
    }
}

fn render_system(world: &World, renderer: &Renderer) {
    for entity in world.query::<(Position, Sprite)>() {
        renderer.draw_sprite(
            entity.sprite.texture,
            entity.position.x,
            entity.position.y
        )
    }
}

// Game setup
fn setup(world: &mut World) {
    // Create player
    world.spawn()
        .with(Position { x: 100.0, y: 100.0 })
        .with(Velocity { dx: 0.0, dy: 0.0 })
        .with(Sprite { 
            texture: "player.png",
            width: 32.0,
            height: 32.0
        })
}

// Game loop
fn update(world: &mut World, input: &Input, dt: f32) {
    movement_system(world)
    physics_system(world, dt)
}

fn render(world: &World, renderer: &Renderer) {
    renderer.clear()
    render_system(world, renderer)
    renderer.present()
}
```

## 2D Game Development

### Basic 2D Game

```windjammer
@game Platformer2D

// Player component
struct Player {
    speed: f32,
    jump_force: f32
}

// Setup
fn setup(world: &mut World) {
    // Create player
    world.spawn()
        .with(Position { x: 100.0, y: 100.0 })
        .with(Velocity { dx: 0.0, dy: 0.0 })
        .with(Sprite { texture: "player.png", width: 32.0, height: 32.0 })
        .with(Player { speed: 200.0, jump_force: 400.0 })
    
    // Create platforms
    for i in 0..10 {
        world.spawn()
            .with(Position { x: i * 64.0, y: 400.0 })
            .with(Sprite { texture: "platform.png", width: 64.0, height: 32.0 })
    }
}

// Input handling
fn handle_input(world: &mut World, input: &Input) {
    for entity in world.query::<(Player, Velocity)>() {
        if input.key_pressed(Key::Left) {
            entity.velocity.dx = -entity.player.speed
        } else if input.key_pressed(Key::Right) {
            entity.velocity.dx = entity.player.speed
        } else {
            entity.velocity.dx = 0.0
        }
        
        if input.key_just_pressed(Key::Space) {
            entity.velocity.dy = -entity.player.jump_force
        }
    }
}

// Physics
fn physics_system(world: &mut World, dt: f32) {
    let gravity = 980.0
    
    for entity in world.query::<Velocity>() {
        entity.velocity.dy += gravity * dt
    }
}
```

### 2D Camera

```windjammer
struct Camera2D {
    x: f32,
    y: f32,
    zoom: f32
}

fn camera_follow_player(world: &World, camera: &mut Camera2D) {
    for entity in world.query::<(Player, Position)>() {
        camera.x = entity.position.x - screen_width() / 2.0
        camera.y = entity.position.y - screen_height() / 2.0
    }
}
```

## 3D Game Development

### Basic 3D Game

```windjammer
@game Game3D

struct Mesh3D {
    model: String,
    material: Material
}

struct Transform3D {
    position: Vec3,
    rotation: Quat,
    scale: Vec3
}

fn setup(world: &mut World) {
    // Create player
    world.spawn()
        .with(Transform3D {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::identity(),
            scale: Vec3::one()
        })
        .with(Mesh3D {
            model: "player.obj",
            material: Material::default()
        })
    
    // Add lighting
    world.spawn()
        .with(DirectionalLight {
            direction: Vec3::new(-1.0, -1.0, -1.0),
            color: Color::white(),
            intensity: 1.0
        })
}
```

### 3D Camera

```windjammer
struct Camera3D {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    fov: f32
}

fn camera_system(world: &World, camera: &mut Camera3D) {
    for entity in world.query::<(Player, Transform3D)>() {
        camera.position = entity.transform.position + Vec3::new(0.0, 5.0, -10.0)
        camera.target = entity.transform.position
    }
}
```

## Physics Integration

### 2D Physics (Rapier2D)

```windjammer
struct RigidBody2D {
    body_type: BodyType,  // Dynamic, Static, Kinematic
    mass: f32,
    friction: f32
}

struct Collider2D {
    shape: ColliderShape,  // Box, Circle, Capsule
    is_sensor: bool
}

fn setup_physics(world: &mut World) {
    // Dynamic object (player)
    world.spawn()
        .with(Position { x: 100.0, y: 100.0 })
        .with(RigidBody2D {
            body_type: BodyType::Dynamic,
            mass: 1.0,
            friction: 0.5
        })
        .with(Collider2D {
            shape: ColliderShape::Box { width: 32.0, height: 32.0 },
            is_sensor: false
        })
    
    // Static object (ground)
    world.spawn()
        .with(Position { x: 0.0, y: 400.0 })
        .with(RigidBody2D {
            body_type: BodyType::Static,
            mass: 0.0,
            friction: 0.8
        })
        .with(Collider2D {
            shape: ColliderShape::Box { width: 800.0, height: 32.0 },
            is_sensor: false
        })
}
```

### 3D Physics (Rapier3D)

```windjammer
struct RigidBody3D {
    body_type: BodyType,
    mass: f32,
    friction: f32
}

struct Collider3D {
    shape: ColliderShape3D,  // Box, Sphere, Capsule, Mesh
    is_sensor: bool
}
```

## Audio System

```windjammer
struct AudioSource {
    sound: String,
    volume: f32,
    looping: bool
}

struct AudioListener {
    position: Vec3
}

fn play_sound(audio: &AudioSystem, sound: &str) {
    audio.play(sound)
}

fn play_music(audio: &AudioSystem, music: &str) {
    audio.play_music(music, true)  // looping
}

// 3D spatial audio
fn spatial_audio_system(world: &World, audio: &AudioSystem) {
    for entity in world.query::<(AudioSource, Transform3D)>() {
        audio.set_position(entity.audio_source.sound, entity.transform.position)
    }
}
```

## Input Handling

```windjammer
fn input_system(input: &Input) {
    // Keyboard
    if input.key_pressed(Key::W) {
        // Move forward
    }
    
    if input.key_just_pressed(Key::Space) {
        // Jump (once per press)
    }
    
    if input.key_just_released(Key::Shift) {
        // Stop sprinting
    }
    
    // Mouse
    let mouse_pos = input.mouse_position()
    if input.mouse_button_pressed(MouseButton::Left) {
        // Shoot
    }
    
    let mouse_delta = input.mouse_delta()
    // Use for camera rotation
}
```

## Game Loop

```windjammer
@game MyGame

fn init() -> GameState {
    let world = World::new()
    setup(&mut world)
    GameState { world }
}

fn update(state: &mut GameState, input: &Input, dt: f32) {
    handle_input(&mut state.world, input)
    physics_system(&mut state.world, dt)
    movement_system(&mut state.world)
    collision_system(&mut state.world)
}

fn render(state: &GameState, renderer: &Renderer) {
    renderer.clear(Color::black())
    render_system(&state.world, renderer)
    ui_system(&state.world, renderer)
    renderer.present()
}
```

## Building Games

### Native Build

```bash
wj build game.wj --output ./game
cd game
cargo run --release
```

### WASM Build

```bash
wj build game.wj --target wasm --output ./game_web
cd game_web
wasm-pack build --target web

# Serve with Windjammer's built-in dev server
wj run ../examples/dev_server.wj
```

### Mobile Build (via Tauri)

```bash
wj build game.wj --target mobile --output ./game_mobile
cd game_mobile
# iOS
cargo tauri ios build
# Android
cargo tauri android build
```

## Performance Tips

### 1. Use ECS Properly

Group components that are accessed together:
```windjammer
// Good: Components accessed together
for entity in world.query::<(Position, Velocity)>() {
    // Fast iteration
}

// Avoid: Separate queries
for entity in world.query::<Position>() {
    let velocity = world.get::<Velocity>(entity.id)  // Slow lookup
}
```

### 2. Batch Rendering

Render similar objects together:
```windjammer
// Group by texture
for texture in textures {
    for entity in world.query_with_texture(texture) {
        renderer.draw(entity)
    }
}
```

### 3. Use Object Pools

Reuse entities instead of spawning/despawning:
```windjammer
struct Bullet {
    active: bool
}

// Reuse inactive bullets
for entity in world.query::<Bullet>() {
    if !entity.bullet.active {
        entity.bullet.active = true
        entity.position = spawn_position
        break
    }
}
```

### 4. Profile Your Game

```bash
cargo build --release
cargo flamegraph --bin game
```

## Best Practices

### 1. Separate Logic and Data

Components = Data only
Systems = Logic only

### 2. Use Events

Decouple systems with events:
```windjammer
struct CollisionEvent {
    entity_a: EntityId,
    entity_b: EntityId
}

fn collision_system(world: &World, events: &mut EventQueue) {
    // Detect collisions
    events.send(CollisionEvent { ... })
}

fn damage_system(world: &mut World, events: &EventQueue) {
    for event in events.read::<CollisionEvent>() {
        // Handle collision
    }
}
```

### 3. Keep Systems Small

One system = one responsibility

### 4. Test Your Systems

```windjammer
#[test]
fn test_movement_system() {
    let mut world = World::new()
    let entity = world.spawn()
        .with(Position { x: 0.0, y: 0.0 })
        .with(Velocity { dx: 10.0, dy: 0.0 })
    
    movement_system(&mut world)
    
    let pos = world.get::<Position>(entity)
    assert_eq!(pos.x, 10.0)
}
```

## Examples

See `examples/game/` for a complete game example (coming soon).

## Resources

- [Bevy ECS](https://bevyengine.org/) - Inspiration for ECS design
- [Rapier Physics](https://rapier.rs/) - Physics engine
- [wgpu](https://wgpu.rs/) - Graphics API

## Next Steps

1. **Learn ECS**: Understand Entity-Component-System pattern
2. **Start Small**: Build a simple 2D game first
3. **Add Physics**: Integrate Rapier for realistic movement
4. **Polish**: Add audio, particles, UI
5. **Optimize**: Profile and improve performance

## See Also

- [UI Framework API](./UI_FRAMEWORK_API.md)
- [Examples Guide](./EXAMPLES_GUIDE.md)
- [Main Documentation](../README.md)

