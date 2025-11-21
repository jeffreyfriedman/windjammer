# Windjammer UI Component Examples

This directory contains example `.wj` component files demonstrating the Windjammer UI framework.

## Examples

### counter.wj
A simple counter component demonstrating:
- State management with reactive variables
- Event handlers
- Dynamic text interpolation
- Multiple functions

### Usage

To compile a component:

```bash
# Compile to WASM
wj build counter.wj --target wasm --output ./counter_output

# The output will include:
# - src/lib.rs (generated Rust code)
# - Cargo.toml (project configuration)
# - index.html (HTML harness)
# - README.md (build instructions)
```

### Component Syntax

Windjammer supports two component syntax styles:

#### 1. Minimal Syntax (Recommended)

```windjammer
// State
count: int = 0

// Functions
fn increment() {
    count = count + 1
}

// View
view {
    button(on_click: increment) {
        "Count: {count}"
    }
}
```

#### 2. Advanced Syntax (Escape Hatch)

```windjammer
@component
struct Counter {
    count: int = 0
}

impl Counter {
    fn increment(&mut self) {
        self.count += 1
    }
    
    fn render(&self) -> VNode {
        // ... VNode construction
    }
}
```

## Features

### State Management
- Reactive variables automatically become `Signal<T>`
- Computed values with `@computed` decorator
- Automatic dependency tracking

### View Syntax
- JSX-like element syntax
- Static and dynamic attributes
- Event handlers with `on_*` prefix
- Text interpolation with `{variable}`
- Conditionals with `if/else`
- Loops with `for item in items`

### Lifecycle Hooks
- `@on_mount` - Called when component mounts
- `@on_destroy` - Called when component unmounts
- `@on_update` - Called when component updates

## Building and Running

1. **Compile the component:**
   ```bash
   wj build counter.wj --target wasm --output ./counter_output
   ```

2. **Build the WASM:**
   ```bash
   cd counter_output
   wasm-pack build --target web
   ```

3. **Serve and test:**
   ```bash
   python3 -m http.server 8080
   # Open http://localhost:8080 in your browser
   ```

## Architecture

The component compiler transforms `.wj` files through several stages:

1. **Parse** - Lexical analysis and AST construction
2. **Analyze** - Dependency tracking and reactive variable detection
3. **Transform** - Convert reactive vars to signals
4. **Generate** - Produce optimized Rust code with WASM bindings

The result is zero-JavaScript reactive components that compile to efficient WebAssembly.

