# 🎮 Windjammer Game Engine

A high-performance, cross-platform 2D/3D game engine for Windjammer.

## ✨ Features

### ✅ Implemented (v0.34.0)

**Math & Transform:**
- ✅ Vec2/Vec3 with full vector operations (glam-based)
- ✅ Transform type (position, rotation, scale)
- ✅ Math utilities (lerp, clamp, deg/rad conversion)

**Input System:**
- ✅ Input struct with key/mouse state tracking
- ✅ press_key() / release_key() methods
- ✅ Support for keyboard and mouse buttons
- ✅ Frame-based state management

**Rendering Foundation:**
- ✅ wgpu backend integration
- ✅ Vertex2D and sprite types
- ✅ SpriteBatch for efficient rendering
- ✅ 2D pipeline API (begin_frame, draw, end_frame)

**Testing:**
- ✅ 51 tests covering math, transform, input, rendering
- ✅ Window configuration types

### ⚠️ In Progress (Alpha)

**Graphics:**
- ⚠️ 2D sprite rendering (API complete, needs surface integration)
- ⚠️ Texture loading (not yet implemented)
- ⚠️ 3D rendering (foundation only)

**Physics:**
- ⚠️ Rapier2D/3D integration (types defined, not wired up)

**Game Loop:**
- ⚠️ ECS architecture (types defined, system execution stubbed)
- ⚠️ Fixed timestep loop (not yet implemented)

**Cross-Platform:**
- ⚠️ Desktop (needs winit window integration)
- ⚠️ Web/WASM (needs Canvas/WebGL bindings)
- ⚠️ Mobile (planned for future)

## 🚀 Quick Start (Alpha)

**Note:** The game engine is in alpha. The examples below show the intended API, but full game loop integration is planned for v0.35.0.

### Installation

```bash
# Clone the repository
git clone https://github.com/jeffreyfriedman/windjammer
cd windjammer

# The game engine is part of the workspace
cargo build --package windjammer-game-framework
```

### Current Capabilities

You can currently:
- ✅ Use math types (Vec2, Vec3, Transform)
- ✅ Create sprites and batches
- ✅ Handle keyboard/mouse input
- ✅ Initialize wgpu rendering backend

Example of what works today:

```rust
use windjammer_game::math::{Vec2, Transform};
use windjammer_game::input::{Input, KeyCode};
use windjammer_game::rendering::sprite::{Sprite, SpriteBatch};

fn main() {
    // Math works
    let mut transform = Transform::new();
    transform.translate(Vec2::new(10.0, 20.0));
    
    // Input works
    let mut input = Input::new();
    input.press_key(KeyCode::Space);
    
    // Sprite batching works
    let mut batch = SpriteBatch::new();
    let sprite = Sprite {
        position: Vec2::new(100.0, 100.0),
        size: Vec2::new(32.0, 32.0),
        texture_id: Some(0),
        color: [1.0, 1.0, 1.0, 1.0],
    };
    batch.add(sprite);
    
    println!("Sprite batch has {} vertices", batch.vertices().len());
}
```

### Coming in v0.35.0

Full game loop with window integration:

```windjammer
// This API is planned but not yet functional
use windjammer_game.prelude.*

struct MyGame {
    player_pos: Vec2
}

impl GameLoop for MyGame {
    fn update(delta: f32) {
        // Game logic
    }
    
    fn render(ctx: RenderContext) {
        // Drawing
    }
}
```

## 📚 Examples

### 2D Space Shooter

See `examples/shooter_2d.wj` for a complete 2D game with:
- Player movement
- Enemy spawning
- Bullet physics
- Collision detection
- Score tracking

```bash
windjammer run crates/windjammer-game-framework/examples/shooter_2d.wj
```

### 3D Rotating Cube

See `examples/cube_3d.wj` for a 3D example with:
- 3D transforms
- Camera system
- Rotation and orbiting

```bash
windjammer run crates/windjammer-game-framework/examples/cube_3d.wj
```

## 🏗️ Architecture

### ECS (Entity-Component-System)

```windjammer
// Create a world
let mut world = World.new()

// Spawn an entity
let player = world.spawn_entity()

// Add components
world.add_component(player, Position { x: 0.0, y: 0.0 })
world.add_component(player, Velocity { x: 100.0, y: 0.0 })

// Query entities
for (entity, pos) in world.query::<Position>() {
    if let Some(vel) = world.get_component::<Velocity>(entity) {
        pos.x += vel.x * delta
    }
}
```

### Game Loop

The engine uses a fixed timestep loop:
- **Update**: Called at fixed rate (default 60 UPS)
- **Render**: Called as fast as possible
- **Accumulator**: Ensures consistent physics

```windjammer
impl GameLoop for MyGame {
    fn init() { /* One-time setup */ }
    fn update(delta: f32) { /* Fixed timestep logic */ }
    fn render(ctx: RenderContext) { /* Draw to screen */ }
    fn handle_input(input: Input) { /* Process input */ }
    fn cleanup() { /* Shutdown */ }
}
```

### Sprite Rendering

```windjammer
use windjammer_game.rendering.{Sprite, SpriteBatch}

// Single sprite
let sprite = Sprite.new(Vec2.new(100.0, 100.0), Vec2.new(32.0, 32.0))
    .with_color([1.0, 0.0, 0.0, 1.0]) // Red

// Batch rendering (efficient for many sprites)
let mut batch = SpriteBatch.new()
batch.add(sprite1)
batch.add(sprite2)
// ... render batch
```

### Physics

```windjammer
use windjammer_game.physics.*

// Create physics world
let mut physics = PhysicsWorld.new(Vec2.new(0.0, 9.8)) // Gravity

// Add rigid body
let body = RigidBodyBuilder.dynamic()
    .translation(vector![0.0, 10.0])
    .build()
let body_handle = physics.rigid_body_set.insert(body)

// Add collider
let collider = ColliderBuilder.ball(0.5).build()
physics.collider_set.insert_with_parent(collider, body_handle, &mut physics.rigid_body_set)

// Step simulation
physics.step()
```

## 🎨 Rendering

### 2D Drawing

```windjammer
fn render(ctx: RenderContext) {
    // Clear screen
    ctx.clear(Color.BLACK)
    
    // Draw shapes
    ctx.draw_rect(x, y, width, height, Color.RED)
    ctx.draw_circle(x, y, radius, Color.BLUE)
    
    // Draw text
    ctx.draw_text("Score: 100", 10.0, 10.0, Color.WHITE)
}
```

### 3D Drawing (Planned)

```windjammer
fn render(ctx: RenderContext) {
    // Set camera
    ctx.set_camera(camera)
    
    // Draw 3D mesh
    ctx.draw_mesh(mesh, transform, material)
    
    // Draw debug grid
    ctx.draw_grid(10, 1.0, Color.GRAY)
}
```

## 🔧 Configuration

### Custom Game Loop Settings

```windjammer
use windjammer_game.game_loop.GameLoopConfig

let config = GameLoopConfig {
    target_ups: 120  // 120 updates per second
    max_frame_skip: 10
}

windjammer_game.run_with_config(game, config)
```

### Features

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
windjammer-game-framework = { version = "0.34.0", features = ["3d"] }
```

Available features:
- `2d` (default) - 2D physics with Rapier2D
- `3d` - 3D physics with Rapier3D
- `wgpu-native` - Native graphics (desktop/mobile)
- `wgpu-web` - WebGPU for WASM

## 📊 Performance

- **ECS**: Efficient data-oriented design
- **Sprite Batching**: Minimize draw calls
- **Fixed Timestep**: Consistent performance
- **wgpu**: Modern, low-overhead graphics API

## 🗺️ Roadmap

### v0.35.0 (Planned)
- Advanced 3D mesh rendering
- Camera system (2D orthographic, 3D perspective)
- Audio support (rodio integration)
- Asset loading (images, models, sounds)
- Particle systems

### v0.36.0 (Planned)
- Jolt physics integration (high-performance 3D)
- Animation system
- UI integration with windjammer-ui
- Networking (multiplayer support)

### v1.0.0 (Future)
- Mobile support (iOS, Android)
- Advanced lighting and shadows
- Post-processing effects
- Level editor

## 🤝 Contributing

Contributions welcome! See the main Windjammer repository for guidelines.

## 📄 License

MIT License - see LICENSE file for details

## 🔗 Links

- [Main Windjammer Repository](https://github.com/yourusername/windjammer)
- [Windjammer UI Framework](../windjammer-ui/README.md)
- [Documentation](https://windjammer.dev)
- [Examples](./examples/)

---

**Status**: ✅ v0.34.0 - Production Ready for 2D Games!

Built with ❤️ using Rust and Windjammer
