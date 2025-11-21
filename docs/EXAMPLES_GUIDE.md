# Windjammer Examples Guide

## Overview

This guide covers all Windjammer examples, from basic language features to advanced applications.

## UI Framework Examples

### Counter (`examples/components/counter.wj`)

**Description**: A simple reactive counter demonstrating automatic reactivity.

**Features**:
- State management
- Event handlers
- Text interpolation
- Automatic DOM updates

**Code**:
```windjammer
count: int = 0

fn increment() {
    count = count + 1
}

fn decrement() {
    count = count - 1
}

fn reset() {
    count = 0
}

view {
    div(class: "counter-app") {
        h1 { "Windjammer Counter" }
        div(class: "counter-display") {
            "Count: ${count}"
        }
        div(class: "counter-controls") {
            button(class: "btn btn-decrement", on_click: decrement) { "-" }
            button(class: "btn btn-reset", on_click: reset) { "Reset" }
            button(class: "btn btn-increment", on_click: increment) { "+" }
        }
    }
}
```

**How to Run**:
```bash
# Compile
wj build examples/components/counter.wj --target wasm --output ./counter_app

# Build WASM
cd counter_app
wasm-pack build --target web

# Serve with Windjammer dev server
wj run ../examples/dev_server.wj

# Open browser
open http://localhost:8080/index.html
```

**What to Test**:
- Click "+" to increment
- Click "-" to decrement
- Click "Reset" to reset to 0
- Observe instant UI updates

### TODO (Advanced) (`examples/components/todo.wj`)

**Description**: Advanced TODO app with lists and filtering (future implementation).

**Status**: Placeholder for future features

## Language Feature Examples

### 01 - Basics (`examples/01_basics/main.wj`)

**Topics**: Variables, functions, control flow

```bash
wj build examples/01_basics/main.wj --output ./output
cd output && cargo run
```

### 02 - Structs (`examples/02_structs/main.wj`)

**Topics**: Struct definitions, methods, constructors

```bash
wj build examples/02_structs/main.wj --output ./output
cd output && cargo run
```

### 03 - Enums (`examples/03_enums/main.wj`)

**Topics**: Enum definitions, pattern matching

```bash
wj build examples/03_enums/main.wj --output ./output
cd output && cargo run
```

### 04 - Traits (`examples/04_traits/main.wj`)

**Topics**: Trait definitions, implementations

```bash
wj build examples/04_traits/main.wj --output ./output
cd output && cargo run
```

### 05 - Modern Features (`examples/05_modern/main.wj`)

**Topics**: Modern Rust features in Windjammer

```bash
wj build examples/05_modern/main.wj --output ./output
cd output && cargo run
```

## Standard Library Examples

### 06 - Stdlib Basics (`examples/06_stdlib/main.wj`)

**Topics**: Using standard library modules

```bash
wj build examples/06_stdlib/main.wj --output ./output
cd output && cargo run
```

### 41 - JSON (`examples/41_json/main.wj`)

**Topics**: JSON parsing and serialization

```bash
wj build examples/41_json/main.wj --output ./output
cd output && cargo run
```

### 42 - HTTP Client (`examples/42_http_client/main.wj`)

**Topics**: Making HTTP requests

```bash
wj build examples/42_http_client/main.wj --output ./output
cd output && cargo run
```

### 46 - HTTP Server (`examples/46_http_server/main.wj`)

**Topics**: Building HTTP servers

```bash
wj build examples/46_http_server/main.wj --output ./output
cd output && cargo run
```

### 48 - Filesystem (`examples/48_filesystem/main.wj`)

**Topics**: File I/O operations

```bash
wj build examples/48_filesystem/main.wj --output ./output
cd output && cargo run
```

## Advanced Examples

### TaskFlow (`examples/taskflow/`)

**Description**: Full-featured task management API

**Features**:
- REST API with authentication
- Database integration
- JWT tokens
- Rate limiting
- Middleware
- Comprehensive error handling

**Structure**:
```
taskflow/
├── src/
│   ├── main.wj              # Entry point
│   ├── config.wj            # Configuration
│   ├── auth/                # Authentication
│   ├── db/                  # Database layer
│   ├── handlers/            # Request handlers
│   ├── middleware/          # Middleware
│   ├── models/              # Data models
│   └── utils/               # Utilities
└── benches/                 # Benchmarks
```

**How to Run**:
```bash
wj build examples/taskflow/windjammer/src/main.wj --output ./taskflow_app
cd taskflow_app
cargo run
```

### wjfind (`examples/wjfind/`)

**Description**: Fast file search tool (ripgrep alternative)

**Features**:
- Regex search
- Gitignore support
- Parallel search
- Replace functionality
- Performance benchmarks

**How to Run**:
```bash
wj build examples/wjfind/src/main.wj --output ./wjfind_app
cd wjfind_app
cargo run -- "pattern" /path/to/search
```

### wschat (`examples/wschat/`)

**Description**: WebSocket chat server

**Features**:
- Real-time messaging
- Room management
- Direct messages
- Presence tracking
- Authentication
- Rate limiting
- Metrics

**How to Run**:
```bash
wj build examples/wschat/src/main.wj --output ./wschat_app
cd wschat_app
cargo run
```

## Testing Examples

### 32 - Testing (`examples/32_testing/main.wj`)

**Topics**: Writing tests in Windjammer

```bash
wj build examples/32_testing/main.wj --output ./output
cd output && cargo test
```

### 35 - Test Decorator (`examples/35_test_decorator/main.wj`)

**Topics**: Using `@test` decorator

```bash
wj build examples/35_test_decorator/main.wj --output ./output
cd output && cargo test
```

## Generic Examples

### 17 - Generics (`examples/17_generics_test/main.wj`)

**Topics**: Generic types and functions

```bash
wj build examples/17_generics_test/main.wj --output ./output
cd output && cargo run
```

### 23 - Turbofish (`examples/23_turbofish_test/main.wj`)

**Topics**: Turbofish syntax `::<T>`

```bash
wj build examples/23_turbofish_test/main.wj --output ./output
cd output && cargo run
```

### 24 - Trait Bounds (`examples/24_trait_bounds/main.wj`)

**Topics**: Generic trait bounds

```bash
wj build examples/24_trait_bounds/main.wj --output ./output
cd output && cargo run
```

### 25 - Where Clauses (`examples/25_where_clauses/main.wj`)

**Topics**: Where clause syntax

```bash
wj build examples/25_where_clauses/main.wj --output ./output
cd output && cargo run
```

### 26 - Associated Types (`examples/26_associated_types/main.wj`)

**Topics**: Trait associated types

```bash
wj build examples/26_associated_types/main.wj --output ./output
cd output && cargo run
```

## Common Patterns

### Building and Running

**Standard Build**:
```bash
wj build <file.wj> --output ./output
cd output && cargo run
```

**WASM Build** (for UI components):
```bash
wj build <component.wj> --target wasm --output ./output
cd output
wasm-pack build --target web

# Serve with Windjammer dev server
wj run ../examples/dev_server.wj
```

**Testing**:
```bash
wj build <file.wj> --output ./output
cd output && cargo test
```

**Benchmarking**:
```bash
wj build <file.wj> --output ./output
cd output && cargo bench
```

### Checking for Errors

```bash
wj check <file.wj>
```

### Watching for Changes

```bash
wj watch <file.wj>
```

## Example Categories

### By Difficulty

**Beginner**:
- 01_basics
- 02_structs
- 03_enums
- hello_world
- counter (UI)

**Intermediate**:
- 04_traits
- 06_stdlib
- 41_json
- 42_http_client
- 46_http_server

**Advanced**:
- taskflow
- wjfind
- wschat
- 24_trait_bounds
- 26_associated_types

### By Topic

**Language Features**:
- 01-05, 17, 23-26, 28-30, 33-34, 37-38, 40

**Standard Library**:
- 06-07, 39, 41-51

**Web Development**:
- 42_http_client
- 46_http_server
- 47_simple_http_server
- taskflow
- wschat

**UI Development**:
- examples/components/counter.wj
- examples/components/todo.wj

**Tools**:
- wjfind
- cli_tool

**Testing**:
- 32_testing
- 35_test_decorator

## Tips

### 1. Start Simple

Begin with `01_basics` and work your way up.

### 2. Read the Code

All examples are well-commented. Read the source to understand patterns.

### 3. Modify and Experiment

Copy examples and modify them to learn.

### 4. Check Generated Rust

Look at the generated Rust code to understand transpilation:
```bash
wj build example.wj --output ./output
cat output/src/main.rs
```

### 5. Use `wj check`

Quickly check for errors without building:
```bash
wj check example.wj
```

### 6. Run Tests

Many examples include tests:
```bash
cd output && cargo test
```

## Troubleshooting

### Build Errors

**Issue**: Compilation fails
**Solution**: Check syntax, run `wj check` first

### Runtime Errors

**Issue**: Program panics
**Solution**: Check generated Rust code, add error handling

### WASM Issues

**Issue**: WASM doesn't load
**Solution**: Ensure `wasm-pack` is installed, check browser console

### Import Errors

**Issue**: Module not found
**Solution**: Use `::` for module paths, check stdlib availability

## Next Steps

1. **Try the Counter**: Start with the UI counter example
2. **Explore Basics**: Work through examples 01-05
3. **Learn Stdlib**: Try examples 06, 39, 41-51
4. **Build Something**: Use taskflow or wschat as templates
5. **Read API Docs**: See [UI_FRAMEWORK_API.md](./UI_FRAMEWORK_API.md)

## Contributing Examples

Want to add an example? Follow this structure:

```
examples/
└── my_example/
    ├── main.wj          # Main file
    ├── README.md        # Description and usage
    └── tests/           # Tests (optional)
```

## See Also

- [UI Framework API](./UI_FRAMEWORK_API.md)
- [Main Documentation](../README.md)
- [ROADMAP](../ROADMAP.md)

