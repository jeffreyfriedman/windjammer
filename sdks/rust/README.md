# Windjammer Rust SDK

**Zero-cost native bindings for the Windjammer Game Engine**

[![Crates.io](https://img.shields.io/crates/v/windjammer-sdk.svg)](https://crates.io/crates/windjammer-sdk)
[![Documentation](https://docs.rs/windjammer-sdk/badge.svg)](https://docs.rs/windjammer-sdk)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🚀 **Zero-cost abstractions** - Direct access to the Rust framework
- 🎮 **Complete API** - Full access to all Windjammer features
- 📦 **Ergonomic API** - Idiomatic Rust patterns and conventions
- 🔒 **Type-safe** - Compile-time guarantees and safety
- ⚡ **High performance** - Native Rust performance
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs
- 🤖 **AI** - Behavior trees, pathfinding, and steering
- 🎭 **Animation** - Skeletal animation, blending, and IK
- 🎯 **Physics** - Rapier2D and Rapier3D integration
- 🎨 **Rendering** - Deferred rendering, PBR, post-processing
- 🔧 **Optimization** - Automatic batching, culling, LOD, and profiling

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
windjammer-sdk = "0.1"
```

## Quick Start

### Hello World

```rust
use windjammer_sdk::prelude::*;

fn main() {
    let mut app = App::new();
    
    app.add_system(hello_system);
    
    app.run();
}

fn hello_system() {
    println!("Hello, Windjammer!");
}
```

### 2D Sprite Example

```rust
use windjammer_sdk::prelude::*;

fn main() {
    let mut app = App::new();
    
    app.add_startup_system(setup);
    app.add_system(update);
    
    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera
    commands.spawn(Camera2DBundle::default());
    
    // Spawn sprite
    commands.spawn(SpriteBundle {
        texture: asset_server.load("sprite.png"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}

fn update(time: Res<Time>, mut query: Query<&mut Transform, With<Sprite>>) {
    for mut transform in query.iter_mut() {
        transform.rotation += time.delta_seconds();
    }
}
```

### 3D Scene Example

```rust
use windjammer_sdk::prelude::*;

fn main() {
    let mut app = App::new();
    
    app.add_startup_system(setup_3d);
    
    app.run();
}

fn setup_3d(mut commands: Commands) {
    // Spawn camera
    commands.spawn(Camera3DBundle {
        transform: Transform::from_xyz(0.0, 5.0, 10.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    
    // Spawn light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    
    // Spawn cube
    commands.spawn(PbrBundle {
        mesh: Mesh::cube(1.0),
        material: Material::standard(),
        ..default()
    });
}
```

## Features

### Core Features

- **ECS (Entity-Component-System)** - High-performance archetype-based ECS
- **App & Plugins** - Modular application architecture
- **Asset Loading** - Async asset loading with hot-reload
- **Input** - Keyboard, mouse, gamepad support
- **Time** - Delta time, fixed timestep, timers

### 2D Features

- **Sprites** - 2D sprite rendering with batching
- **Tilemaps** - Tile-based maps
- **2D Physics** - Rapier2D integration
- **2D Camera** - Orthographic camera with zoom

### 3D Features

- **3D Rendering** - Deferred rendering pipeline
- **PBR Materials** - Physically-based rendering
- **Skeletal Animation** - GPU-accelerated skinning
- **3D Physics** - Rapier3D integration
- **3D Camera** - First-person, third-person, free camera

### Audio

- **3D Spatial Audio** - Positional audio with doppler effect
- **Audio Mixing** - Hierarchical bus system
- **Audio Effects** - Reverb, echo, filters, distortion
- **Audio Streaming** - For music and large files

### Networking

- **Client-Server** - TCP/UDP networking
- **Entity Replication** - Automatic state synchronization
- **RPCs** - Remote procedure calls

### AI

- **Behavior Trees** - Advanced AI decision making
- **Pathfinding** - A* with navmesh support
- **Steering Behaviors** - Smooth AI movement

### Optimization

- **Automatic Batching** - Draw call reduction
- **Frustum Culling** - Visibility culling
- **LOD System** - Level of detail management
- **Memory Pooling** - Allocation optimization
- **Profiler** - Built-in performance profiler

## Examples

Run examples with:

```bash
cargo run --example hello_world
cargo run --example sprite_demo
cargo run --example 3d_scene --features 3d
```

## Documentation

- [API Documentation](https://docs.rs/windjammer-sdk)
- [User Guide](https://windjammer.dev/guide)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/rust/examples)
- [Tutorials](https://windjammer.dev/tutorials)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

