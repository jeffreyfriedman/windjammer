# WebAssembly Game Example - Conway's Game of Life

An interactive Game of Life implementation built with Windjammer, demonstrating:

- WebAssembly compilation for browser execution
- High-performance game loop
- Canvas rendering
- Pattern matching for game rules
- Memory-safe array operations
- Performance monitoring with `@timing` decorator

## Building

```bash
windjammer build --target wasm
cd output
wasm-pack build --target web
```

## Running

```bash
# Serve the example
cd www
python3 -m http.server 8080
```

Then open http://localhost:8080 in your browser.

## Features Demonstrated

### WebAssembly Bindings
- `@wasm_bindgen` decorator for exposing Rust/Windjammer code to JavaScript
- Seamless interop with web APIs
- Zero-copy memory sharing with JavaScript

### Pattern Matching
The game rules are implemented using clean pattern matching:

```go
let next_cell = match (cell, live_neighbors) {
    (true, x) if x < 2 => false,      // Dies from underpopulation
    (true, 2) | (true, 3) => true,    // Lives on
    (true, x) if x > 3 => false,      // Dies from overpopulation
    (false, 3) => true,               // Reproduction
    (otherwise, _) => otherwise,      // No change
}
```

### Performance
- Runs at 60 FPS in the browser
- `@timing` decorator tracks tick performance
- Efficient memory layout for the cell grid
- SIMD-friendly array operations (when compiled with optimizations)

### Ownership & Safety
- No manual memory management
- Bounds checking on array access
- Safe mutation patterns
- All with zero runtime cost!

## Game Controls

- **Click**: Toggle individual cells
- **Space**: Pause/Resume
- **R**: Randomize the grid
- **C**: Clear the grid
- **S**: Step forward one generation (when paused)

## Patterns to Try

The game includes classic patterns:
- Gliders
- Blinkers
- Glider guns
- Still lifes

## Performance Stats

The game displays real-time statistics:
- Generation number
- Frames per second (FPS)
- Number of alive cells

With Rust's zero-cost abstractions, this implementation achieves native-like performance in the browser while maintaining complete memory safety.

