# WASM Canvas Support

Guide for running Windjammer games in the browser using Canvas2D or WebGL.

## Overview

The Windjammer Game Framework can target WebAssembly to run games in the browser. This document describes how to use Canvas2D and WebGL for rendering.

## Architecture

```
Windjammer Game
     ↓
  wgpu (Graphics API)
     ↓
WebGPU (Browser)
     ↓
Canvas Element
```

## Setup

### 1. Add WASM Dependencies

```toml
[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "Document",
    "Window",
    "HtmlCanvasElement",
    "WebGl2RenderingContext",
    "CanvasRenderingContext2d",
] }
js-sys = "0.3"
```

### 2. Create HTML Template

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Windjammer Game</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            background: #222;
        }
        canvas {
            border: 2px solid #444;
        }
    </style>
</head>
<body>
    <canvas id="game-canvas" width="800" height="600"></canvas>
    <script type="module">
        import init from './pkg/your_game.js';
        
        async function run() {
            await init();
        }
        
        run();
    </script>
</body>
</html>
```

### 3. Configure for WASM

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main_wasm() {
    // Set panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Get canvas element
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let canvas = document
        .get_element_by_id("game-canvas")
        .expect("no canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("not a canvas");
    
    // Initialize game with canvas
    let game = MyGame::new(canvas);
    
    // Run game loop
    start_game_loop(game);
}
```

## Canvas2D Rendering

For simple 2D games, you can use Canvas2D:

```rust
use web_sys::CanvasRenderingContext2d;

pub struct Canvas2DRenderer {
    context: CanvasRenderingContext2d,
}

impl Canvas2DRenderer {
    pub fn new(canvas: HtmlCanvasElement) -> Self {
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        
        Self { context }
    }
    
    pub fn clear(&self, color: &str) {
        self.context.set_fill_style(&color.into());
        self.context.fill_rect(0.0, 0.0, 800.0, 600.0);
    }
    
    pub fn draw_rect(&self, x: f64, y: f64, w: f64, h: f64, color: &str) {
        self.context.set_fill_style(&color.into());
        self.context.fill_rect(x, y, w, h);
    }
    
    pub fn draw_circle(&self, x: f64, y: f64, radius: f64, color: &str) {
        self.context.begin_path();
        self.context.arc(x, y, radius, 0.0, 2.0 * std::f64::consts::PI).unwrap();
        self.context.set_fill_style(&color.into());
        self.context.fill();
    }
}
```

## WebGL Rendering

For more advanced graphics, use WebGL (via wgpu):

```rust
// wgpu automatically uses WebGL/WebGPU when targeting WASM
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::GL | wgpu::Backends::BROWSER_WEBGPU,
    ..Default::default()
});

// Create surface from canvas
let surface = instance.create_surface_from_canvas(canvas).unwrap();
```

## Game Loop

Use `requestAnimationFrame` for smooth animation:

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}

pub fn start_game_loop(mut game: impl GameLoop + 'static) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        // Update game
        game.update(1.0 / 60.0);
        
        // Render game
        // game.render();
        
        // Schedule next frame
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));
    
    request_animation_frame(g.borrow().as_ref().unwrap());
}
```

## Building for WASM

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build your game
wasm-pack build --target web

# Serve and test
python3 -m http.server 8080
# Open http://localhost:8080
```

## Input Handling

Handle browser events:

```rust
use web_sys::{KeyboardEvent, MouseEvent};

// Keyboard
let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
    let key = event.key();
    // Handle key press
}) as Box<dyn FnMut(_)>);

window
    .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
    .unwrap();
closure.forget();

// Mouse
let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
    let x = event.client_x();
    let y = event.client_y();
    // Handle mouse click
}) as Box<dyn FnMut(_)>);

canvas
    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
    .unwrap();
closure.forget();
```

## Performance Tips

1. **Use WebGPU when available** - Much faster than WebGL
2. **Batch draw calls** - Minimize state changes
3. **Use sprite atlases** - Reduce texture bindings
4. **Profile with browser DevTools** - Find bottlenecks
5. **Optimize WASM size** - Use `wasm-opt` for smaller binaries

## Example: Simple WASM Game

See `examples/wasm_game/` for a complete working example.

## Troubleshooting

### Canvas not found
- Ensure canvas ID matches in HTML and Rust code
- Check that script loads after DOM is ready

### Black screen
- Check browser console for errors
- Verify WebGL/WebGPU support in browser
- Test with simpler Canvas2D first

### Poor performance
- Profile with browser DevTools
- Reduce draw calls
- Use sprite batching
- Consider WebGPU instead of WebGL

## See Also

- [wgpu WASM Guide](https://github.com/gfx-rs/wgpu/wiki/Running-on-the-Web-with-WebGPU-and-WebGL)
- [wasm-bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)


