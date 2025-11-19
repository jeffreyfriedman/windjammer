# Windjammer JavaScript/TypeScript SDK

**JavaScript and TypeScript bindings for the Windjammer Game Engine**

[![npm version](https://badge.fury.io/js/windjammer-sdk.svg)](https://badge.fury.io/js/windjammer-sdk)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.2-blue.svg)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🎯 **Full TypeScript Support** - Complete type definitions
- 🌐 **Browser & Node.js** - Works in both environments
- 🎮 **Complete API** - Full access to all Windjammer features
- 🚀 **High Performance** - Native Rust backend via WebAssembly
- 📦 **Easy Installation** - Simple npm install
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs
- 🤖 **AI** - Behavior trees, pathfinding, and steering
- 🎭 **Animation** - Skeletal animation, blending, and IK
- 🎯 **Physics** - Rapier2D and Rapier3D integration
- 🎨 **Rendering** - Deferred rendering, PBR, post-processing

## Installation

```bash
npm install windjammer-sdk
```

Or with yarn:

```bash
yarn add windjammer-sdk
```

## Quick Start

### Hello World (TypeScript)

```typescript
import { App } from 'windjammer-sdk';

const app = new App();

app.addSystem(() => {
  console.log('Hello, Windjammer!');
});

app.run();
```

### Hello World (JavaScript)

```javascript
import { App } from 'windjammer-sdk';

const app = new App();

app.addSystem(() => {
  console.log('Hello, Windjammer!');
});

app.run();
```

### 2D Sprite Example

```typescript
import { App, Vec2, Sprite, Camera2D } from 'windjammer-sdk';

const app = new App();

app.addStartupSystem(() => {
  // Spawn camera
  const camera = new Camera2D(new Vec2(0, 0), 1.0);
  
  // Spawn sprite
  const sprite = new Sprite({
    texture: 'sprite.png',
    position: new Vec2(0, 0),
    size: new Vec2(100, 100)
  });
});

app.addSystem((time) => {
  // Update sprites each frame
});

app.run();
```

### 3D Scene Example

```typescript
import { App, Vec3, Camera3D, PointLight, Mesh, Material } from 'windjammer-sdk';

const app = new App();

app.addStartupSystem(() => {
  // Spawn camera
  const camera = new Camera3D({
    position: new Vec3(0, 5, 10),
    lookAt: new Vec3(0, 0, 0),
    fov: 60
  });
  
  // Spawn light
  const light = new PointLight({
    position: new Vec3(4, 8, 4),
    intensity: 1500
  });
  
  // Spawn cube
  const cube = Mesh.cube(1.0);
  const material = Material.standard();
});

app.run();
```

## API Reference

### Core Classes

#### `App`
Main application class.

```typescript
const app = new App();
app.addSystem(system);
app.addStartupSystem(system);
app.run();
```

#### `Vec2`, `Vec3`, `Vec4`
Vector math classes.

```typescript
const v2 = new Vec2(x, y);
const v3 = new Vec3(x, y, z);
const v4 = new Vec4(x, y, z, w);

// Operations
const sum = v3.add(other);
const diff = v3.sub(other);
const scaled = v3.mul(scalar);
const len = v3.length();
const normalized = v3.normalize();
```

#### `Transform`
Position, rotation, and scale.

```typescript
const transform = new Transform({
  position: new Vec3(0, 0, 0),
  rotation: new Vec3(0, 0, 0),
  scale: new Vec3(1, 1, 1)
});
```

### 2D Classes

#### `Sprite`
2D sprite component.

```typescript
const sprite = new Sprite({
  texture: 'path/to/texture.png',
  position: new Vec2(0, 0),
  size: new Vec2(100, 100)
});
```

#### `Camera2D`
2D orthographic camera.

```typescript
const camera = new Camera2D(
  new Vec2(0, 0),  // position
  1.0              // zoom
);
```

### 3D Classes

#### `Camera3D`
3D perspective camera.

```typescript
const camera = new Camera3D({
  position: new Vec3(0, 5, 10),
  lookAt: new Vec3(0, 0, 0),
  fov: 60
});
```

#### `Mesh`
3D mesh component.

```typescript
const cube = Mesh.cube(1.0);
const sphere = Mesh.sphere(1.0, 32);
const plane = Mesh.plane(10.0);
```

#### `Material`
PBR material.

```typescript
const material = Material.standard({
  albedo: [1.0, 1.0, 1.0],
  metallic: 0.5,
  roughness: 0.5
});
```

## TypeScript Support

The SDK includes full TypeScript definitions:

```typescript
import { App, Vec3, System } from 'windjammer-sdk';

const updateSystem: System = (time) => {
  console.log(`Delta: ${time.deltaSeconds}`);
};

const app = new App();
app.addSystem(updateSystem);
```

## Examples

Run examples with:

```bash
npm run build
node examples/hello-world.js
node examples/sprite-demo.js
node examples/3d-scene.js
```

## Testing

Run tests with:

```bash
npm test
```

With coverage:

```bash
npm run test:coverage
```

## Building

Build the TypeScript source:

```bash
npm run build
```

## Documentation

- [API Documentation](https://windjammer.dev/docs/javascript)
- [User Guide](https://windjammer.dev/guide)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/javascript/examples)
- [Tutorials](https://windjammer.dev/tutorials)

## Browser Usage

The SDK can be used in the browser via a bundler (webpack, vite, etc.):

```typescript
import { App, Vec2 } from 'windjammer-sdk';

const app = new App();
// Your game code here
app.run();
```

Or via a CDN:

```html
<script type="module">
  import { App } from 'https://unpkg.com/windjammer-sdk@latest/dist/index.js';
  
  const app = new App();
  app.run();
</script>
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

