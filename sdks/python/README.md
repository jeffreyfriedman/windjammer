# Windjammer Python SDK

**Python bindings for the Windjammer Game Engine**

[![PyPI version](https://badge.fury.io/py/windjammer-sdk.svg)](https://badge.fury.io/py/windjammer-sdk)
[![Python versions](https://img.shields.io/pypi/pyversions/windjammer-sdk.svg)](https://pypi.org/project/windjammer-sdk/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🐍 **Pythonic API** - Idiomatic Python interface
- 🎮 **Complete Access** - Full access to all Windjammer features
- 🚀 **High Performance** - Native Rust backend via FFI
- 📦 **Easy Installation** - Simple pip install
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs
- 🤖 **AI** - Behavior trees, pathfinding, and steering
- 🎭 **Animation** - Skeletal animation, blending, and IK
- 🎯 **Physics** - Rapier2D and Rapier3D integration
- 🎨 **Rendering** - Deferred rendering, PBR, post-processing
- 🔧 **Optimization** - Automatic batching, culling, LOD, and profiling

## Installation

```bash
pip install windjammer-sdk
```

Or install from source:

```bash
git clone https://github.com/windjammer/windjammer.git
cd windjammer/sdks/python
pip install -e .
```

## Quick Start

### Hello World

```python
from windjammer_sdk import App

def main():
    app = App()
    
    @app.system
    def hello_system():
        print("Hello, Windjammer!")
    
    app.run()

if __name__ == "__main__":
    main()
```

### 2D Sprite Example

```python
from windjammer_sdk import App, Vec2, Transform, Sprite

def main():
    app = App()
    
    @app.startup
    def setup(commands, assets):
        # Spawn camera
        commands.spawn_camera_2d()
        
        # Spawn sprite
        sprite = Sprite(
            texture=assets.load("sprite.png"),
            position=Vec2(0, 0)
        )
        commands.spawn(sprite)
    
    @app.system
    def update(time, sprites):
        for sprite in sprites:
            sprite.transform.rotation += time.delta_seconds
    
    app.run()

if __name__ == "__main__":
    main()
```

### 3D Scene Example

```python
from windjammer_sdk import App, Vec3, Camera3D, PointLight, Mesh, Material

def main():
    app = App()
    
    @app.startup
    def setup_3d(commands):
        # Spawn camera
        camera = Camera3D(
            position=Vec3(0, 5, 10),
            look_at=Vec3(0, 0, 0)
        )
        commands.spawn(camera)
        
        # Spawn light
        light = PointLight(
            position=Vec3(4, 8, 4),
            intensity=1500.0
        )
        commands.spawn(light)
        
        # Spawn cube
        cube = Mesh.cube(size=1.0)
        material = Material.standard()
        commands.spawn_mesh(cube, material)
    
    app.run()

if __name__ == "__main__":
    main()
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
python examples/hello_world.py
python examples/sprite_demo.py
python examples/3d_scene.py
```

## API Reference

### Core Classes

#### `App`
Main application class.

```python
app = App()
app.run()
```

#### `Vec2`, `Vec3`, `Vec4`
Vector math classes.

```python
v2 = Vec2(x, y)
v3 = Vec3(x, y, z)
v4 = Vec4(x, y, z, w)
```

#### `Transform`
Position, rotation, and scale.

```python
transform = Transform(
    position=Vec3(0, 0, 0),
    rotation=Vec3(0, 0, 0),
    scale=Vec3(1, 1, 1)
)
```

### 2D Classes

#### `Sprite`
2D sprite component.

```python
sprite = Sprite(
    texture="path/to/texture.png",
    position=Vec2(0, 0),
    size=Vec2(100, 100)
)
```

#### `Camera2D`
2D orthographic camera.

```python
camera = Camera2D(
    position=Vec2(0, 0),
    zoom=1.0
)
```

### 3D Classes

#### `Camera3D`
3D perspective camera.

```python
camera = Camera3D(
    position=Vec3(0, 5, 10),
    look_at=Vec3(0, 0, 0),
    fov=60.0
)
```

#### `Mesh`
3D mesh component.

```python
cube = Mesh.cube(size=1.0)
sphere = Mesh.sphere(radius=1.0, subdivisions=32)
plane = Mesh.plane(size=10.0)
```

#### `Material`
PBR material.

```python
material = Material.standard(
    albedo=(1.0, 1.0, 1.0),
    metallic=0.5,
    roughness=0.5
)
```

## Type Hints

The SDK includes full type hints for better IDE support:

```python
from windjammer_sdk import App, Vec3
from typing import List

def process_positions(positions: List[Vec3]) -> Vec3:
    return Vec3.average(positions)
```

## Testing

Run tests with:

```bash
pytest tests/
```

With coverage:

```bash
pytest --cov=windjammer_sdk tests/
```

## Documentation

- [API Documentation](https://windjammer.dev/docs/python)
- [User Guide](https://windjammer.dev/guide)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/python/examples)
- [Tutorials](https://windjammer.dev/tutorials)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

