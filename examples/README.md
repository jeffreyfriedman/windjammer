# Windjammer Examples

This directory contains examples demonstrating Windjammer's features and capabilities.

## Directory Structure

### 📁 Applications (Real-World Projects)

Production-ready applications showcasing Windjammer's capabilities:

- **`taskflow/`** - Full-featured task management REST API with authentication, database, middleware
- **`wjfind/`** - High-performance file search tool (ripgrep alternative)
- **`wschat/`** - WebSocket-based real-time chat server with metrics
- **`cli_tool/`** - Command-line application with argument parsing
- **`http_server/`** - RESTful HTTP server with routing and middleware
- **`optimization_validation/`** - Performance optimization validation suite
- **`wasm_game/`** - WebAssembly game with rendering
- **`wasm_hello/`** - WebAssembly "Hello World" example
- **`applications/`** - Additional smaller applications:
  - `counter_complete/` - Complete reactive counter with WASM
  - `dev_server/` - Development HTTP server
  - `form_validation/` - Form validation with UI framework
  - `platformer_2d/` - 2D platformer game with ECS
  - `ui_counter_simple/` - Simple UI counter
  - `wasm_dev_server/` - WASM development server
  - `components/` - UI component examples
  - `game/` - Game framework examples

### 📁 Syntax Tests (Language Feature Demonstrations)

Simple examples demonstrating specific language features:

- **`syntax_tests/`** - Numbered examples (01-51, 99) covering:
  - Basic syntax (01-05)
  - Standard library (06-20, 39-51)
  - Generics and traits (17, 24-30, 33-34, 38)
  - Modules and imports (10, 16, 22)
  - Decorators (35-36, 40)
  - Testing (08, 12, 32)
  - Advanced features (37)

## Running Examples

### Application Examples

```bash
# Build and run a full application
wj build taskflow/windjammer/src/main.wj
wj run taskflow/windjammer/src/main.wj

# Build for WASM
wj build wasm_game/main.wj --target wasm
```

### Syntax Test Examples

```bash
# Run a specific syntax test
wj run syntax_tests/01_basics/main.wj

# Test all examples
wj test syntax_tests/
```

## Example Categories

### Web Development
- `taskflow/` - Full REST API
- `http_server/` - HTTP server basics
- `wschat/` - WebSocket server
- `applications/dev_server/` - Dev server

### Game Development
- `applications/platformer_2d/` - 2D platformer with ECS
- `wasm_game/` - WASM game
- `applications/game/` - Game framework examples

### UI Development
- `applications/counter_complete/` - Reactive UI
- `applications/form_validation/` - Form handling
- `applications/components/` - Reusable components
- `applications/ui_counter_simple/` - Simple counter

### Command-Line Tools
- `wjfind/` - File search tool
- `cli_tool/` - CLI argument parsing

### WebAssembly
- `wasm_game/` - WASM game
- `wasm_hello/` - WASM basics
- `applications/wasm_dev_server/` - WASM dev server

## Testing Examples

All examples are automatically tested in CI. To test locally:

```bash
# Test all examples
make test-examples

# Test specific category
wj test syntax_tests/
wj test applications/

# Test individual example
wj build taskflow/windjammer/src/main.wj --output /tmp/test
```

## Contributing Examples

When adding new examples:

1. **Applications** - Place in root or `applications/` if it's a complete, real-world project
2. **Syntax Tests** - Place in `syntax_tests/` if it demonstrates a specific language feature
3. **Always use subdirectories** - No standalone `.wj` files in the root
4. **Include README.md** - Document what the example demonstrates
5. **Ensure it builds** - Test with `wj build` before committing

## Example Quality Standards

- ✅ All examples should compile without errors
- ✅ Use idiomatic Windjammer (not just transpiled Rust)
- ✅ Include comments explaining key concepts
- ✅ Follow Windjammer style guide
- ✅ No warnings from `wj build`
- ✅ Include README.md for complex examples


