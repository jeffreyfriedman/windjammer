# Windjammer UI Framework

**Cross-platform UI framework and 2D game engine for Windjammer**

Build reactive web, desktop, and mobile applications AND 2D games using clean Windjammer syntax with Rust performance.

## 🎨 Features

### UI Framework
- **Component Model**: `@component` decorator for building UI components
- **Reactive State**: Svelte-inspired fine-grained reactivity (Signal, Computed, Effect)
- **Virtual DOM**: Efficient diffing and patching
- **Server-Side Rendering (SSR)**: Generate HTML on the server, hydrate on the client
- **File-Based Routing**: Automatic route discovery from filesystem
- **Cross-Platform Events**: Unified event system with capturing/bubbling
- **Platform Abstraction**: Write once, run on Web, Desktop (Tauri), Mobile (iOS/Android)

### Game Framework
- **Entity-Component System (ECS)**: Efficient game entity management
- **2D Math**: Vec2, Vec3 with full vector operations
- **2D Physics**: AABB collision, Rigidbody simulation
- **Input Handling**: Keyboard, mouse, touch, gamepad-ready
- **Rendering**: Sprites, shapes, text, colors
- **Time Management**: Delta time, FPS tracking

### Native Capabilities
- **Filesystem**: Permission-based file access
- **GPS/Location**: Latitude, longitude, altitude, accuracy
- **Camera**: Image capture with multiple formats
- **Clipboard**: Read/write text
- **Notifications**: Native system notifications
- **Accelerometer**: Motion sensing (x, y, z)

## 📦 Crates

- **`windjammer-ui`**: Main framework (Rust library)
- **`windjammer-ui-macro`**: Procedural macros (`#[component]`, `#[derive(Props)]`)

## 🚀 Examples

All examples are in **idiomatic Windjammer** (`.wj` files):

### UI Examples
- **`counter.wj`**: Basic component with state
- **`todo_app.wj`**: Full CRUD app with state management
- **`form_validation.wj`**: Form handling with validation rules
- **`ssr_hydration.wj`**: Server-side rendering with client hydration
- **`routing_multi_page.wj`**: Multi-page app with routing
- **`platform_capabilities.wj`**: Native API access (filesystem, GPS, camera, etc)

### Game Examples
- **`shooter_game.wj`**: Space shooter with collision detection
- **`puzzle_game.wj`**: 2048-style puzzle game

## 💻 Current Status

**v0.34.0 - IN DEVELOPMENT**

The framework architecture is complete with 91 passing tests. **Parser integration complete!** 🎉

### ✅ What Works
- Complete Rust library implementation (91 tests passing)
- All core APIs designed and tested
- Comprehensive example code (8 examples in idiomatic Windjammer)
- Cross-platform architecture ready
- **Parser support for glob imports (`use module.*`)**
- **Parser support for braced imports (`use module.{A, B, C}`)**
- **External crate imports (`use windjammer_ui.prelude.*`)**
- **`.wj` → Rust transpilation with auto-generated Cargo.toml**
- **`wj build` command (generates Rust + Cargo.toml with deps)**
- **`wj run` command (transpiles, builds, and runs - partial)**

### 🚧 What's Being Built (Next)
- **Runtime implementation** (print, string interpolation, component macros)
- `#[component]` macro implementation (generate constructors, render methods)
- Web runtime (browser DOM integration via WASM)
- Desktop runtime (Tauri integration)
- Game runtime (rendering backends: Canvas/WebGL)
- WASM packaging with wasm-pack

## 📖 Usage Example

```windjammer
// counter.wj
use windjammer_ui.prelude.*
use windjammer_ui.vdom.{VElement, VNode, VText}

@component
struct Counter {
    count: int
}

impl Counter {
    fn render() -> VNode {
        VElement.new("div")
            .attr("class", "counter")
            .child(VNode.Element(
                VElement.new("h1").child(VNode.Text(
                    VText.new("Count: {count}")
                ))
            ))
            .child(VNode.Element(
                VElement.new("button")
                    .attr("onclick", "increment")
                    .child(VNode.Text(VText.new("Increment")))
            ))
            .into()
    }
}

fn main() {
    let counter = Counter.new()
    let vnode = counter.render()
    print("Rendered: {vnode:?}")
}
```

## 🎮 Game Example

```windjammer
// shooter_game.wj (excerpt)
use windjammer_ui.game.*

@derive(Debug, Clone)
struct Player {
    position: Vec2
    velocity: Vec2
    health: int
}

@game
struct ShooterGame {
    player: Player
    enemies: [Enemy]
    bullets: [Bullet]
}

impl GameLoop for ShooterGame {
    fn update(delta: f32) {
        // Update game state
        player.position += player.velocity * delta
        
        // Check collisions
        for bullet in bullets {
            for enemy in enemies {
                if check_collision(bullet, enemy) {
                    enemy.health -= 25
                }
            }
        }
    }
    
    fn render(ctx: RenderContext) {
        ctx.clear(Color.BLACK)
        ctx.draw_rect(player.position.x, player.position.y, 40.0, 40.0, Color.GREEN)
        // ... render enemies, bullets, UI
    }
}
```

## 🏗️ Architecture

### Layers
1. **User Code** (`.wj` files): Clean Windjammer syntax
2. **Transpiler** (in progress): `.wj` → Rust
3. **Framework** (Rust): This crate (`windjammer-ui`)
4. **Runtime** (in progress): Platform-specific implementations

### Platform Support Matrix

| Feature | Web (WASM/JS) | Desktop (Tauri) | Mobile (iOS/Android) | Status |
|---------|---------------|-----------------|----------------------|--------|
| Components | ✅ | ✅ | ✅ | Ready |
| Reactivity | ✅ | ✅ | ✅ | Ready |
| Virtual DOM | ✅ | ✅ | ✅ | Ready |
| SSR | ✅ | - | - | Ready |
| Routing | ✅ | ✅ | ✅ | Ready |
| Events | ✅ | ✅ | ✅ | Ready |
| Filesystem | Browser API | Native | Native | Ready |
| GPS | Geolocation API | Native | Native | Ready |
| Camera | Media API | Native | Native | Ready |
| 2D Games | Canvas/WebGL | GPU | GPU | Ready |
| Runtime | 🚧 In Progress | 🚧 In Progress | 🚧 In Progress | Next |

## 🔧 Development

```bash
# Run tests
cargo test -p windjammer-ui

# Check lints
cargo clippy -p windjammer-ui

# Format code
cargo fmt -p windjammer-ui
```

## 📋 Remaining Work (TODOs for Next Session)

### Critical Path (Make Examples Runnable)
1. ⏳ **Complete .wj file parser integration** for UI framework syntax
2. ⏳ **Create `wj run` command** for executing .wj files
3. ⏳ **Web runtime**: Connect WebRenderer to actual browser DOM
4. ⏳ **Game runtime**: Complete game loop with actual rendering
5. ⏳ **WASM packaging**: wasm-pack integration for browser deployment
6. ⏳ **Desktop runtime**: Complete Tauri integration

### Developer Experience
7. ⏳ **Update LSP**: Add completion for @component, @game, UI framework types
8. ⏳ **Update LSP**: Add hover docs for windjammer_ui APIs
9. ⏳ **Update MCP**: Add tools for UI component generation
10. ⏳ **Update MCP**: Add game entity scaffolding tool
11. ⏳ **Update MCP**: Add SSR/routing analysis tools

### Documentation
12. ⏳ **Update design doc**: Clarify Rust vs Windjammer (library vs user code)

## 🎯 Design Philosophy

### Idiomatic Windjammer
- **`use` (not `import`)**: `use windjammer_ui.prelude.*`
- **`.` separators (not `::`)**:  `use std.http`
- **`@decorators`**: `@component`, `@derive(Debug, Clone)`
- **Implicit `self`**: `position += velocity` (compiler adds `self.`)
- **String interpolation**: `"Score: {score}"` (not `format!`)
- **Auto-borrow inference**: No `&` or `&mut` in user code
- **Clean types**: `int`, `string`, `[T]` (not `i32`, `String`, `Vec<T>`)

### Rust Library, Windjammer Apps
- **The framework itself** (`windjammer-ui`, `windjammer-ui-macro`): Written in Rust
- **Proc macros**: Must be Rust (compile-time code generation)
- **User applications**: Written in Windjammer (`.wj` files)
- **Examples**: All `.wj` files showing how users write apps

## 📊 Testing

- **91 tests passing**
  - 30 UI framework tests
  - 21 game framework tests  
  - 13 routing tests
  - 10 capability tests
  - 9 SSR tests
  - 5 event propagation tests
  - 3 platform tests

## 📦 Building for Web (WASM)

### Prerequisites

```bash
cargo install wasm-pack
```

### Build for Web

```bash
cd crates/windjammer-ui
./build-wasm.sh
```

This creates three build variants:
- `pkg/web/` - For vanilla HTML/JS (use with `<script type="module">`)
- `pkg/bundler/` - For webpack/rollup/vite
- `pkg/nodejs/` - For Node.js applications

### Using in HTML

```html
<!DOCTYPE html>
<html>
<head>
    <title>Windjammer UI App</title>
</head>
<body>
    <div id="app"></div>
    
    <script type="module">
        import init from './pkg/web/windjammer_ui.js';
        await init();
    </script>
</body>
</html>
```

### Using with Vite/Webpack

```javascript
import init from 'windjammer-ui';

async function main() {
    await init();
    // Your app code here
}

main();
```

## 🌟 Inspiration

- **Svelte**: Simplicity and reactivity model
- **Dioxus**: Cross-platform Rust UI
- **Tauri**: Desktop app framework
- **Unity/Godot**: Game development workflows
- **Bevy**: ECS architecture

## 📄 License

Same as main Windjammer project (see root LICENSE file)

## 🤝 Contributing

See main Windjammer CONTRIBUTING.md

## 🔗 Links

- [Main Windjammer Repo](../../)
- [Design Document](../../docs/design/windjammer-ui.md)
- [Multi-Target Codegen](../../docs/design/multi-target-codegen.md)
- [ROADMAP](../../ROADMAP.md)

---

**Status**: Active development for v0.34.0  
**Completed**: Parser integration, implicit self, array types, `wj run`, web runtime, WASM packaging  
**In Progress**: LSP/MCP updates, game runtime, desktop integration

