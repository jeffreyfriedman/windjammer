# Windjammer Quick Start Guide

**Get started with Windjammer in 5 minutes!**

---

## Choose Your Language

Windjammer supports 12 programming languages. Pick your favorite:

- [Rust](#rust-quickstart) - Zero-cost, native performance
- [Python](#python-quickstart) - Rapid prototyping, 15M developers
- [JavaScript/TypeScript](#javascript-quickstart) - Web games, 17M developers
- [C#](#csharp-quickstart) - Unity-like API, 6M developers
- [C++](#cpp-quickstart) - AAA performance, 4M developers
- [Go](#go-quickstart) - Modern, concurrent
- [Java](#java-quickstart) - Enterprise, Android
- [Kotlin](#kotlin-quickstart) - Modern JVM, Android
- [Lua](#lua-quickstart) - Embedded scripting
- [Swift](#swift-quickstart) - iOS, macOS
- [Ruby](#ruby-quickstart) - Elegant, expressive

---

## Rust Quickstart

### 1. Install

```bash
cargo new my_game
cd my_game
cargo add windjammer-sdk
```

### 2. Hello World

```rust
use windjammer_sdk::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_system(hello);
    app.run();
}

fn hello() {
    println!("Hello, Windjammer!");
}
```

### 3. Run

```bash
cargo run
```

### 4. Next Steps

- [2D Sprite Game](#2d-sprite-game-rust)
- [3D Scene](#3d-scene-rust)
- [Full Tutorial](TUTORIALS.md)

---

## Python Quickstart

### 1. Install

```bash
pip install windjammer-sdk
```

### 2. Hello World

```python
from windjammer_sdk import App

app = App()

@app.system
def hello():
    print("Hello, Windjammer!")

app.run()
```

### 3. Run

```bash
python game.py
```

### 4. Next Steps

- [2D Sprite Game](#2d-sprite-game-python)
- [3D Scene](#3d-scene-python)
- [Full Tutorial](TUTORIALS.md)

---

## JavaScript Quickstart

### 1. Install

```bash
npm install windjammer-sdk
```

### 2. Hello World

```javascript
import { App } from 'windjammer-sdk';

const app = new App();

app.addSystem(() => {
    console.log("Hello, Windjammer!");
});

app.run();
```

### 3. Run

```bash
node game.js
```

### 4. Next Steps

- [2D Sprite Game](#2d-sprite-game-javascript)
- [3D Scene](#3d-scene-javascript)
- [Full Tutorial](TUTORIALS.md)

---

## C# Quickstart

### 1. Install

```bash
dotnet new console -n MyGame
cd MyGame
dotnet add package Windjammer.SDK
```

### 2. Hello World

```csharp
using Windjammer.SDK;

var app = new App();

app.AddSystem(() =>
{
    Console.WriteLine("Hello, Windjammer!");
});

app.run();
```

### 3. Run

```bash
dotnet run
```

### 4. Next Steps

- [2D Sprite Game](#2d-sprite-game-csharp)
- [3D Scene](#3d-scene-csharp)
- [Unity Migration Guide](UNITY_MIGRATION.md)

---

## C++ Quickstart

### 1. Install

```bash
# Using vcpkg
vcpkg install windjammer-sdk

# Or CMake
cmake -B build
cmake --build build
```

### 2. Hello World

```cpp
#include <windjammer/windjammer.hpp>

int main() {
    wj::App app;
    
    app.add_system([]() {
        std::cout << "Hello, Windjammer!\n";
    });
    
    app.run();
    return 0;
}
```

### 3. Run

```bash
./build/my_game
```

### 4. Next Steps

- [2D Sprite Game](#2d-sprite-game-cpp)
- [3D Scene](#3d-scene-cpp)
- [Full Tutorial](TUTORIALS.md)

---

## 2D Sprite Game (Rust)

Create a simple 2D game with a moving sprite:

```rust
use windjammer_sdk::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_startup_system(setup);
    app.add_system(update);
    app.run();
}

fn setup() {
    // Create 2D camera
    Camera2D::new(Vec2::ZERO, 1.0);
    
    // Create sprite
    Sprite {
        texture: "player.png",
        position: Vec2::ZERO,
        size: Vec2::new(64.0, 64.0),
    };
}

fn update(time: Res<Time>, input: Res<Input>, mut query: Query<&mut Sprite>) {
    for mut sprite in query.iter_mut() {
        // Move with arrow keys
        let mut velocity = Vec2::ZERO;
        if input.key_pressed(KeyCode::Left) { velocity.x -= 1.0; }
        if input.key_pressed(KeyCode::Right) { velocity.x += 1.0; }
        if input.key_pressed(KeyCode::Up) { velocity.y += 1.0; }
        if input.key_pressed(KeyCode::Down) { velocity.y -= 1.0; }
        
        sprite.position += velocity * 200.0 * time.delta;
    }
}
```

---

## 2D Sprite Game (Python)

```python
from windjammer_sdk import App, Camera2D, Sprite, Vec2, Input, KeyCode

app = App()

@app.startup
def setup():
    Camera2D(Vec2(0, 0), 1.0)
    Sprite(texture="player.png", position=Vec2(0, 0), size=Vec2(64, 64))

@app.system
def update(time, sprites):
    for sprite in sprites:
        velocity = Vec2(0, 0)
        if Input.key_pressed(KeyCode.LEFT): velocity.x -= 1
        if Input.key_pressed(KeyCode.RIGHT): velocity.x += 1
        if Input.key_pressed(KeyCode.UP): velocity.y += 1
        if Input.key_pressed(KeyCode.DOWN): velocity.y -= 1
        
        sprite.position += velocity * 200 * time.delta

app.run()
```

---

## 3D Scene (Rust)

Create a 3D scene with lighting and post-processing:

```rust
use windjammer_sdk::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_startup_system(setup);
    app.run();
}

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
    
    // Cube with PBR material
    Mesh::cube(1.0).with_material(Material {
        albedo: Color::new(0.8, 0.2, 0.2, 1.0),
        metallic: 0.8,
        roughness: 0.2,
        emissive: Color::new(0.5, 0.1, 0.1, 1.0),
    });
    
    // Ground plane
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
}
```

---

## Common Patterns

### Player Movement

```rust
fn move_player(input: Res<Input>, time: Res<Time>, mut query: Query<&mut Transform>) {
    for mut transform in query.iter_mut() {
        let speed = 5.0;
        let mut direction = Vec3::ZERO;
        
        if input.key_pressed(KeyCode::W) { direction.z -= 1.0; }
        if input.key_pressed(KeyCode::S) { direction.z += 1.0; }
        if input.key_pressed(KeyCode::A) { direction.x -= 1.0; }
        if input.key_pressed(KeyCode::D) { direction.x += 1.0; }
        
        transform.translation += direction.normalize() * speed * time.delta;
    }
}
```

### Collision Detection

```rust
fn check_collisions(
    mut query: Query<(&Transform, &Collider)>,
    mut events: EventWriter<CollisionEvent>,
) {
    for (transform_a, collider_a) in query.iter() {
        for (transform_b, collider_b) in query.iter() {
            if collider_a.intersects(collider_b, transform_a, transform_b) {
                events.send(CollisionEvent { a, b });
            }
        }
    }
}
```

### Audio Playback

```rust
fn play_sound(audio: Res<AudioServer>) {
    let sound = audio.load("explosion.wav");
    audio.play(sound, AudioSettings {
        volume: 0.8,
        spatial: true,
        position: Vec3::new(10.0, 0.0, 0.0),
    });
}
```

### Networking

```rust
fn setup_multiplayer() {
    // Server
    let mut server = NetworkServer::start(7777)?;
    
    // Client
    let mut client = NetworkClient::connect("127.0.0.1:7777")?;
    
    // Send message
    client.send_message(&PlayerInput { x: 1.0, y: 0.0 }, Channel::Unreliable);
}
```

---

## Next Steps

### Tutorials
- [Complete Platformer Tutorial](TUTORIALS.md#platformer)
- [FPS Game Tutorial](TUTORIALS.md#fps)
- [Multiplayer Game Tutorial](TUTORIALS.md#multiplayer)

### Documentation
- [API Reference](API_REFERENCE.md) - Complete API documentation
- [Feature Showcase](FEATURE_SHOWCASE.md) - All features explained
- [Cookbook](COOKBOOK.md) - Common patterns and recipes

### Migration Guides
- [Unity to Windjammer](UNITY_MIGRATION.md)
- [Godot to Windjammer](GODOT_MIGRATION.md)

### Community
- [Discord Server](https://discord.gg/windjammer)
- [Forum](https://forum.windjammer.dev)
- [GitHub](https://github.com/windjammer/windjammer)

---

## Performance Tips

1. **Use ECS Queries** - Iterate over components, not entities
2. **Enable Optimizations** - Batching, culling, LOD are automatic
3. **Profile First** - Use built-in profiler before optimizing
4. **Batch Draw Calls** - Group objects with same material
5. **Use Object Pooling** - Reuse objects instead of creating new ones

---

## Troubleshooting

### Game Won't Start
- Check that all assets exist
- Verify SDK is installed correctly
- Look for errors in console

### Low FPS
- Enable runtime optimizations
- Reduce draw calls (batching)
- Lower post-processing quality
- Use LOD system

### Assets Not Loading
- Check file paths are correct
- Verify assets are in correct format
- Check asset server is initialized

---

## Get Help

- **Discord**: [discord.gg/windjammer](https://discord.gg/windjammer)
- **Forum**: [forum.windjammer.dev](https://forum.windjammer.dev)
- **GitHub Issues**: [github.com/windjammer/windjammer/issues](https://github.com/windjammer/windjammer/issues)
- **Email**: support@windjammer.dev

---

**Ready to build your game? Let's go! 🚀**


