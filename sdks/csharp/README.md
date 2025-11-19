# Windjammer C# SDK

**C# bindings for the Windjammer Game Engine - Unity-like API**

[![NuGet](https://img.shields.io/nuget/v/Windjammer.SDK.svg)](https://www.nuget.org/packages/Windjammer.SDK/)
[![.NET](https://img.shields.io/badge/.NET-8.0-blue.svg)](https://dotnet.microsoft.com/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🎮 **Unity-like API** - Familiar API for Unity developers
- 🔒 **Type-safe** - Full C# type safety with nullable reference types
- 🚀 **High Performance** - Native Rust backend via P/Invoke
- 📦 **Easy Installation** - Simple NuGet package
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs
- 🤖 **AI** - Behavior trees, pathfinding, and steering
- 🎭 **Animation** - Skeletal animation, blending, and IK
- 🎯 **Physics** - Rapier2D and Rapier3D integration
- 🎨 **Rendering** - Deferred rendering, PBR, post-processing
- 🔧 **Optimization** - Automatic batching, culling, LOD, and profiling

## Installation

Install via NuGet Package Manager:

```bash
dotnet add package Windjammer.SDK
```

Or via Package Manager Console:

```powershell
Install-Package Windjammer.SDK
```

## Quick Start

### Hello World

```csharp
using Windjammer.SDK;

var app = new App();

app.AddSystem(() =>
{
    Console.WriteLine("Hello, Windjammer!");
});

app.Run();
```

### 2D Sprite Example

```csharp
using Windjammer.SDK;

var app = new App();

app.AddStartupSystem(() =>
{
    // Spawn camera
    var camera = new Camera2D(new Vector2(0, 0), 1.0f);
    
    // Spawn sprite
    var sprite = new Sprite
    {
        Texture = "sprite.png",
        Position = new Vector2(0, 0),
        Size = new Vector2(100, 100)
    };
});

app.AddSystem((Time time) =>
{
    // Update sprites each frame
});

app.Run();
```

### 3D Scene Example

```csharp
using Windjammer.SDK;

var app = new App();

app.AddStartupSystem(() =>
{
    // Spawn camera
    var camera = new Camera3D
    {
        Position = new Vector3(0, 5, 10),
        LookAt = new Vector3(0, 0, 0),
        Fov = 60f
    };
    
    // Spawn light
    var light = new PointLight
    {
        Position = new Vector3(4, 8, 4),
        Intensity = 1500f
    };
    
    // Spawn cube
    var cube = Mesh.CreateCube(1.0f);
    var material = Material.CreateStandard();
});

app.Run();
```

## API Reference

### Core Classes

#### `App`
Main application class.

```csharp
var app = new App();
app.AddSystem(system);
app.AddStartupSystem(system);
app.Run();
```

#### `Vector2`, `Vector3`, `Vector4`
Vector math structs.

```csharp
var v2 = new Vector2(x, y);
var v3 = new Vector3(x, y, z);
var v4 = new Vector4(x, y, z, w);

// Operations
var sum = v3 + other;
var diff = v3 - other;
var scaled = v3 * scalar;
var len = v3.Length();
var normalized = v3.Normalized();
```

#### `Transform`
Position, rotation, and scale.

```csharp
var transform = new Transform
{
    Position = new Vector3(0, 0, 0),
    Rotation = new Vector3(0, 0, 0),
    Scale = new Vector3(1, 1, 1)
};
```

### 2D Classes

#### `Sprite`
2D sprite component.

```csharp
var sprite = new Sprite
{
    Texture = "path/to/texture.png",
    Position = new Vector2(0, 0),
    Size = new Vector2(100, 100)
};
```

#### `Camera2D`
2D orthographic camera.

```csharp
var camera = new Camera2D(
    new Vector2(0, 0),  // position
    1.0f                // zoom
);
```

### 3D Classes

#### `Camera3D`
3D perspective camera.

```csharp
var camera = new Camera3D
{
    Position = new Vector3(0, 5, 10),
    LookAt = new Vector3(0, 0, 0),
    Fov = 60f
};
```

#### `Mesh`
3D mesh component.

```csharp
var cube = Mesh.CreateCube(1.0f);
var sphere = Mesh.CreateSphere(1.0f, 32);
var plane = Mesh.CreatePlane(10.0f);
```

#### `Material`
PBR material.

```csharp
var material = Material.CreateStandard(
    albedo: new Vector3(1, 1, 1),
    metallic: 0.5f,
    roughness: 0.5f
);
```

## Unity Migration Guide

Windjammer's C# SDK is designed to be familiar to Unity developers:

| Unity | Windjammer |
|-------|-----------|
| `GameObject` | Entity with components |
| `MonoBehaviour` | System functions |
| `Start()` | `AddStartupSystem()` |
| `Update()` | `AddSystem()` |
| `Vector3` | `Vector3` (same!) |
| `Transform` | `Transform` (same!) |
| `Rigidbody` | `RigidBody` |
| `Collider` | `Collider` |

### Example Migration

**Unity:**
```csharp
public class PlayerController : MonoBehaviour
{
    void Start()
    {
        // Initialization
    }
    
    void Update()
    {
        // Per-frame logic
    }
}
```

**Windjammer:**
```csharp
var app = new App();

app.AddStartupSystem(() =>
{
    // Initialization
});

app.AddSystem(() =>
{
    // Per-frame logic
});

app.Run();
```

## Examples

Run examples with:

```bash
cd Examples
dotnet run --project HelloWorld
dotnet run --project SpriteDemo
dotnet run --project Scene3D
```

## Testing

Run tests with:

```bash
cd Windjammer.SDK.Tests
dotnet test
```

## Documentation

- [API Documentation](https://windjammer.dev/docs/csharp)
- [User Guide](https://windjammer.dev/guide)
- [Unity Migration Guide](https://windjammer.dev/unity-migration)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/csharp/Examples)
- [Tutorials](https://windjammer.dev/tutorials)

## Why Choose Windjammer over Unity?

- ✅ **Open Source** - No licensing fees, ever
- ✅ **Rust Performance** - Native Rust backend for maximum speed
- ✅ **Modern Architecture** - ECS-based, not GameObject hierarchy
- ✅ **Automatic Optimization** - Built-in batching, culling, LOD
- ✅ **Multi-Language** - Use C#, Python, JavaScript, Rust, or others
- ✅ **No Runtime Fees** - Deploy anywhere, pay nothing
- ✅ **Better Tooling** - Modern editor and debugging tools

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

