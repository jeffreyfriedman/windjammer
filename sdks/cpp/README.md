# Windjammer C++ SDK

**Modern C++20 bindings for the Windjammer Game Engine**

[![vcpkg](https://img.shields.io/badge/vcpkg-available-blue.svg)](https://vcpkg.io/)
[![C++20](https://img.shields.io/badge/C%2B%2B-20-blue.svg)](https://en.cppreference.com/w/cpp/20)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🚀 **Modern C++20** - Concepts, ranges, coroutines
- 🔒 **Type-safe** - Strong typing with RAII
- ⚡ **Zero-overhead** - Header-only, inline everything
- 📦 **Easy Installation** - CMake, vcpkg, or conan
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs
- 🤖 **AI** - Behavior trees, pathfinding, and steering
- 🎭 **Animation** - Skeletal animation, blending, and IK
- 🎯 **Physics** - Rapier2D and Rapier3D integration
- 🎨 **Rendering** - Deferred rendering, PBR, post-processing

## Installation

### CMake

```cmake
find_package(WindjammerSDK REQUIRED)
target_link_libraries(your_game PRIVATE Windjammer::windjammer_sdk)
```

### vcpkg

```bash
vcpkg install windjammer-sdk
```

### Conan

```bash
conan install windjammer-sdk/0.1.0@
```

## Quick Start

### Hello World

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

### 2D Sprite Example

```cpp
#include <windjammer/windjammer.hpp>

int main() {
    wj::App app;
    
    app.add_startup_system([]() {
        // Spawn camera
        auto camera = wj::Camera2D{{0.0f, 0.0f}, 1.0f};
        
        // Spawn sprite
        auto sprite = wj::Sprite{
            .texture = "sprite.png",
            .position = {0.0f, 0.0f},
            .size = {100.0f, 100.0f}
        };
    });
    
    app.add_system([](const wj::Time& time) {
        // Update sprites each frame
    });
    
    app.run();
    return 0;
}
```

### 3D Scene Example

```cpp
#include <windjammer/windjammer.hpp>

int main() {
    wj::App app;
    
    app.add_startup_system([]() {
        // Spawn camera
        auto camera = wj::Camera3D{
            .position = {0.0f, 5.0f, 10.0f},
            .look_at = {0.0f, 0.0f, 0.0f},
            .fov = 60.0f
        };
        
        // Spawn light
        auto light = wj::PointLight{
            .position = {4.0f, 8.0f, 4.0f},
            .intensity = 1500.0f
        };
        
        // Spawn cube
        auto cube = wj::Mesh::cube(1.0f);
        auto material = wj::Material::standard();
    });
    
    app.run();
    return 0;
}
```

## API Reference

### Core Classes

#### `wj::App`
Main application class.

```cpp
wj::App app;
app.add_system(system);
app.add_startup_system(system);
app.run();
```

#### `wj::Vec2`, `wj::Vec3`, `wj::Vec4`
Vector math classes.

```cpp
auto v2 = wj::Vec2{x, y};
auto v3 = wj::Vec3{x, y, z};
auto v4 = wj::Vec4{x, y, z, w};

// Operations
auto sum = v3 + other;
auto diff = v3 - other;
auto scaled = v3 * scalar;
auto len = v3.length();
auto normalized = v3.normalized();
```

#### `wj::Transform`
Position, rotation, and scale.

```cpp
auto transform = wj::Transform{
    .position = {0.0f, 0.0f, 0.0f},
    .rotation = {0.0f, 0.0f, 0.0f},
    .scale = {1.0f, 1.0f, 1.0f}
};
```

### Modern C++ Features

#### Designated Initializers (C++20)

```cpp
auto sprite = wj::Sprite{
    .texture = "player.png",
    .position = {0.0f, 0.0f},
    .size = {64.0f, 64.0f}
};
```

#### Lambda Systems

```cpp
app.add_system([](const wj::Time& time) {
    // System logic here
});
```

#### RAII Resource Management

```cpp
{
    wj::AudioSource audio{"music.mp3"};
    audio.play();
} // Automatically cleaned up
```

## Building

```bash
mkdir build && cd build
cmake ..
cmake --build .
```

## Examples

```bash
cd build
./examples/hello_world
./examples/sprite_demo
./examples/3d_scene
```

## Documentation

- [API Documentation](https://windjammer.dev/docs/cpp)
- [User Guide](https://windjammer.dev/guide)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/cpp/examples)
- [Tutorials](https://windjammer.dev/tutorials)

## Why C++ for Game Development?

- ✅ **Industry Standard** - Used by AAA studios worldwide
- ✅ **Maximum Performance** - Zero-overhead abstractions
- ✅ **Fine-grained Control** - Memory management, optimization
- ✅ **Modern Features** - C++20 brings huge improvements
- ✅ **Ecosystem** - Vast library ecosystem
- ✅ **Cross-platform** - Windows, Linux, macOS, consoles

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

