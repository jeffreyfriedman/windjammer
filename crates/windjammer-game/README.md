# 🎮 Windjammer Game Engine

A high-performance, cross-platform 2D/3D game engine for Windjammer.

## ✨ Features

### Core
- **ECS Architecture** - Efficient Entity-Component-System
- **Fixed Timestep Loop** - Consistent physics and game logic (60 UPS)
- **Cross-Platform** - Web (WASM), Desktop (Windows/macOS/Linux), Mobile (iOS/Android planned)

### Graphics
- **wgpu Backend** - Modern graphics API supporting:
  - Metal (macOS, iOS)
  - Vulkan (Linux, Android, Windows)
  - DirectX 12 (Windows)
  - WebGPU (Web)
- **2D Sprite Rendering** - Efficient sprite batching
- **3D Support** - Foundation for 3D games (mesh rendering planned)

### Physics
- **2D Physics** - Rapier2D for collision detection and rigid body dynamics
- **3D Physics** - Rapier3D (optional feature)

### Input & Time
- **Input Handling** - Keyboard, mouse, touch support
- **Time Management** - Delta time, frame counting, FPS tracking

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
windjammer-game = "0.34.0"
```

### Your First Game

Create `my_game.wj`:

```windjammer
use windjammer_game.prelude.*

struct MyGame {
    player_pos: Vec2
}

impl GameLoop for MyGame {
    fn init() {
        print("Game started!")
    }
    
    fn update(delta: f32) {
        // Update game logic
        player_pos.x += 100.0 * delta
    }
    
    fn render(ctx: RenderContext) {
        ctx.clear(Color.BLACK)
        ctx.draw_rect(player_pos.x, player_pos.y, 32.0, 32.0, Color.BLUE)
    }
}

fn main() {
    let game = MyGame { player_pos: Vec2.ZERO }
    windjammer_game.run(game)
}
```

### Build & Run

```bash
# Build for native
windjammer build my_game.wj

# Run
windjammer run my_game.wj

# Build for WASM
windjammer build my_game.wj --target wasm
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
windjammer run crates/windjammer-game/examples/shooter_2d.wj
```

### 3D Rotating Cube

See `examples/cube_3d.wj` for a 3D example with:
- 3D transforms
- Camera system
- Rotation and orbiting

```bash
windjammer run crates/windjammer-game/examples/cube_3d.wj
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
windjammer-game = { version = "0.34.0", features = ["3d"] }
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
