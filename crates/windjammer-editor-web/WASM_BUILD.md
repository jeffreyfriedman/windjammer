# Windjammer Editor - WASM Build Guide

This guide explains how to build and run the Windjammer web editor using WebAssembly (WASM).

## Prerequisites

### 1. Install Rust and wasm-pack

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### 2. Install Node.js (Optional, for development server)

```bash
# macOS (using Homebrew)
brew install node

# Or download from https://nodejs.org/
```

## Building the Editor

### Quick Build

Use the provided build script:

```bash
./build.sh
```

This will:
1. Check for wasm-pack installation
2. Build the WASM package for web target
3. Output to `pkg/` directory

### Manual Build

If you prefer to build manually:

```bash
# Build for web (optimized for size)
wasm-pack build --target web --release --out-dir pkg

# Build for development (faster compilation, larger size)
wasm-pack build --target web --dev --out-dir pkg
```

### Build Options

**Target Options:**
- `--target web` - For use in browsers with ES modules
- `--target bundler` - For use with bundlers like webpack
- `--target nodejs` - For use in Node.js

**Profile Options:**
- `--release` - Optimized build (slower compile, smaller size)
- `--dev` - Development build (faster compile, larger size)
- `--profiling` - Profiling build (optimized with debug info)

## Running the Editor

### Option 1: Using the Serve Script

```bash
./serve.sh
```

This starts a Python HTTP server on port 8080.

### Option 2: Using Python HTTP Server

```bash
python3 -m http.server 8080
```

### Option 3: Using Node.js HTTP Server

```bash
# Install http-server globally
npm install -g http-server

# Start server
http-server -p 8080
```

### Option 4: Using a Custom Server

Any static file server will work. Just make sure it serves:
- `index.html`
- `pkg/` directory (WASM files)
- `*.js` files with correct MIME types

## Project Structure

```
windjammer-editor-web/
├── src/
│   ├── lib.rs              # Main WASM entry point
│   ├── engine_bridge.rs    # Game engine WASM bindings
│   ├── editor.rs           # Editor logic
│   ├── compiler_bridge.rs  # Compiler integration
│   └── ...
├── pkg/                    # Generated WASM package (after build)
│   ├── windjammer_editor_web.js
│   ├── windjammer_editor_web_bg.wasm
│   └── ...
├── index.html              # Main HTML file
├── wasm-loader.js          # WASM loader and API
├── webgl-renderer.js       # WebGL 3D renderer
├── styles.css              # Editor styles
├── build.sh                # Build script
├── serve.sh                # Development server script
└── Cargo.toml              # Rust package manifest
```

## WASM API

The editor exposes several WASM APIs:

### Game Engine

```javascript
import { initWasm, createGameEngine, startGameEngine } from './wasm-loader.js';

// Initialize WASM
await initWasm();

// Create engine instance
const canvas = document.getElementById('viewport');
const engine = createGameEngine(canvas);

// Start the engine
startGameEngine();
```

### Scene Management

```javascript
import { createScene, createEntity } from './wasm-loader.js';

// Create a new scene
const scene = createScene('My Scene');

// Create entities
const entity = createEntity(1, 'Player');
entity.set_position(0, 0, 0);

// Add entity to scene
scene.add_entity(entity);

// Serialize scene to JSON
const json = scene.to_json();
console.log(json);
```

## Performance Optimization

### Build Size Optimization

The `Cargo.toml` is already configured for size optimization:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable Link Time Optimization
codegen-units = 1   # Reduce number of codegen units
strip = true        # Strip symbols from binary
```

### Runtime Optimization

1. **Use `--release` builds in production**
   ```bash
   wasm-pack build --target web --release
   ```

2. **Enable WASM streaming compilation**
   ```javascript
   // The browser will compile WASM while downloading
   WebAssembly.compileStreaming(fetch('pkg/windjammer_editor_web_bg.wasm'));
   ```

3. **Use Web Workers for heavy computation**
   ```javascript
   // Offload compilation to a worker thread
   const worker = new Worker('wasm-worker.js');
   ```

### Typical Build Sizes

- **Development build**: ~2-5 MB
- **Release build**: ~500 KB - 1 MB
- **Release + gzip**: ~200-400 KB

## Debugging

### Enable Console Logging

Add this to your `Cargo.toml`:

```toml
[dependencies]
console_error_panic_hook = "0.1"
```

And in your Rust code:

```rust
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
```

### Browser DevTools

1. Open browser DevTools (F12)
2. Go to Sources tab
3. Look for `wasm://` URLs
4. Set breakpoints in WASM code

### WASM Profiling

Use Chrome DevTools Performance tab:

1. Record a performance profile
2. Look for WASM function calls
3. Analyze hot paths

## Troubleshooting

### Build Errors

**Error: `wasm-pack not found`**
```bash
cargo install wasm-pack
```

**Error: `wasm32-unknown-unknown target not found`**
```bash
rustup target add wasm32-unknown-unknown
```

### Runtime Errors

**Error: `Failed to load WASM module`**
- Make sure you're serving files over HTTP (not `file://`)
- Check browser console for CORS errors
- Verify `pkg/` directory exists and contains WASM files

**Error: `Memory access out of bounds`**
- This usually indicates a bug in the Rust code
- Check array indices and pointer arithmetic
- Enable debug assertions: `wasm-pack build --dev`

### Performance Issues

**Slow compilation:**
- Use `--dev` for faster builds during development
- Use `--release` only for production builds

**Large WASM file:**
- Make sure you're using `--release` mode
- Check `Cargo.toml` for optimization settings
- Consider code splitting if your app is very large

## Advanced Topics

### Custom Memory Allocation

```rust
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

### WASM Threading

```bash
# Build with threading support
wasm-pack build --target web --release -- --features wasm-threads
```

### WASM SIMD

```bash
# Build with SIMD support
RUSTFLAGS="-C target-feature=+simd128" wasm-pack build --target web --release
```

## Resources

- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [Rust and WebAssembly Book](https://rustwasm.github.io/docs/book/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [WebAssembly MDN](https://developer.mozilla.org/en-US/docs/WebAssembly)

## Next Steps

1. **Build the editor**: `./build.sh`
2. **Start the server**: `./serve.sh`
3. **Open in browser**: http://localhost:8080
4. **Start creating games!**

For more information, see the main [README.md](README.md).

