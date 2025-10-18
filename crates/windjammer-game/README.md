# Windjammer Game Engine

**High-performance 2D/3D game engine for Windjammer**

Build games with modern graphics APIs, physics, audio, and cross-platform support.

## 🎮 Features

### Graphics
- **Multi-API Support**: Metal (macOS/iOS), Vulkan (cross-platform), DirectX 12 (Windows), WebGPU (web)
- **Powered by wgpu**: Modern, safe graphics abstraction
- **2D & 3D**: Sprites, meshes, materials, cameras
- **Performance**: SIMD-optimized math with `glam`

### Physics
- **2D Physics**: Powered by `rapier2d`
- **3D Physics**: Powered by `rapier3d` (optional)
- **Features**: Rigid bodies, colliders, joints, ray casting

### Audio
- **Multiple Backends**: `rodio` (default) or `kira`
- **Features**: Spatial audio, streaming, effects

### Architecture
- **ECS**: Entity-Component-System for flexible game logic
- **Cross-Platform**: Desktop (Windows, macOS, Linux), Web (WASM)
- **Asset Management**: Loading and caching for textures, sounds, models

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
windjammer-game = "0.34.0"
```

Or in Windjammer code:

```windjammer
use windjammer_game.prelude.*
```

## 🚀 Quick Start

```windjammer
use windjammer_game.prelude.*

@game
struct MyGame {
    player: Entity,
    world: World,
}

impl GameLoop for MyGame {
    fn init() {
        // Initialize game
        world = World.new()
        player = world.spawn()
        world.add_component(player, Position { x: 0.0, y: 0.0 })
        world.add_component(player, Velocity { x: 0.0, y: 0.0 })
    }
    
    fn update(delta: f32) {
        // Update game logic
        for entity in world.entities() {
            if let Some(pos) = world.get_component_mut::<Position>(entity) {
                if let Some(vel) = world.get_component::<Velocity>(entity) {
                    pos.x += vel.x * delta
                    pos.y += vel.y * delta
                }
            }
        }
    }
    
    fn render(ctx: RenderContext) {
        // Render game
        ctx.clear(Vec4.new(0.0, 0.0, 0.0, 1.0))
        
        for entity in world.entities() {
            if let Some(pos) = world.get_component::<Position>(entity) {
                ctx.draw_sprite(sprite, Vec2.new(pos.x, pos.y))
            }
        }
    }
}

fn main() {
    let game = MyGame.new()
    run(game)
}
```

## 🏗️ Architecture

### Modules

- **`ecs`**: Entity-Component-System
- **`math`**: Math types (Vec2, Vec3, Mat4, Quat)
- **`physics`**: Physics simulation (Rapier)
- **`rendering`**: Graphics rendering (wgpu)
- **`audio`**: Audio playback
- **`input`**: Keyboard, mouse, gamepad input
- **`assets`**: Asset loading and management
- **`time`**: Delta time and FPS tracking
- **`window`**: Window creation and management

### Graphics APIs

The engine automatically selects the best graphics API for your platform:

| Platform | Primary API | Fallback |
|----------|-------------|----------|
| macOS | Metal | Vulkan |
| iOS | Metal | - |
| Windows | DirectX 12 | Vulkan |
| Linux | Vulkan | - |
| Web | WebGPU | WebGL 2 |

## 🎯 Examples

### 2D Platformer

```windjammer
use windjammer_game.prelude.*

@game
struct Platformer {
    player: Entity,
    platforms: [Entity],
    physics: PhysicsWorld,
}

impl GameLoop for Platformer {
    fn update(delta: f32) {
        // Handle input
        if input.is_key_pressed(KeyCode.Space) {
            // Jump
        }
        
        // Update physics
        physics.step()
    }
    
    fn render(ctx: RenderContext) {
        // Render game
    }
}
```

### Top-Down Shooter

```windjammer
use windjammer_game.prelude.*

@game
struct Shooter {
    player: Entity,
    enemies: [Entity],
    bullets: [Entity],
}

impl GameLoop for Shooter {
    fn update(delta: f32) {
        // Update player
        // Update enemies
        // Check collisions
    }
    
    fn render(ctx: RenderContext) {
        // Render game
    }
}
```

## 🔧 Features

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
windjammer-game = { version = "0.34.0", features = ["3d", "audio-kira"] }
```

Available features:
- `2d` (default): 2D physics and rendering
- `3d`: 3D physics and rendering
- `audio-rodio` (default): Audio with rodio
- `audio-kira`: Audio with kira (more features)
- `web`: Web/WASM support
- `async`: Async runtime (tokio)

## 📊 Status

**v0.34.0 - Foundation Complete**

### ✅ Implemented
- ECS architecture (18 tests passing)
- Math library (glam integration)
- Physics integration (rapier2d)
- Module structure
- Cross-platform support

### 🚧 In Progress
- wgpu rendering backend
- Game loop implementation
- Asset loading system
- Audio integration
- Working examples

### 🔮 Planned (v0.35.0+)
- 3D rendering
- Advanced physics (soft bodies, cloth)
- Networking (multiplayer)
- Editor tools
- Hot reload

## 🤝 Comparison

| Feature | Windjammer Game | Bevy | Godot | Unity |
|---------|----------------|------|-------|-------|
| Language | Windjammer | Rust | GDScript | C# |
| ECS | ✅ | ✅ | Partial | Partial |
| Graphics | wgpu | wgpu | Vulkan/OpenGL | Custom |
| Physics | Rapier | Rapier | Godot Physics | PhysX |
| Learning Curve | Low | Medium | Low | Medium |
| Compile Time | Fast | Medium | N/A | N/A |
| Memory Safety | ✅ | ✅ | ❌ | ❌ |
| WASM Support | ✅ | ✅ | ✅ | ❌ |

## 📚 Documentation

- [Main Windjammer Docs](../../README.md)
- [Game Engine Design](../../docs/design/windjammer-game.md)
- [Examples](./examples/)
- [API Reference](https://docs.rs/windjammer-game)

## 📄 License

Same as main Windjammer project (see root LICENSE file)

---

**Built with ❤️ for the Windjammer community**

