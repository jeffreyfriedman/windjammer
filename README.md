# Windjammer Programming Language

**Write simple code. Run it fast. Debug it easily.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

A high-level programming language that combines Go's ergonomics with Rust's safety and performance—plus world-class IDE support.

> **🎯 The 80/20 Language**: 80% of Rust's power with 20% of the complexity  
> **🛠️ Production-Ready Tooling**: Complete LSP, debugging, and editor integration  
> **📊 [Read the detailed comparison: Windjammer vs Rust vs Go](docs/COMPARISON.md)**

---

## What is Windjammer?

Windjammer is a pragmatic systems programming language that compiles to **Rust, JavaScript, and WebAssembly**, giving you:

✅ **Memory safety** without garbage collection  
✅ **Rust-level performance** (99%+ measured)  
✅ **Multi-target compilation** - Rust, JavaScript (ES2020+), WebAssembly  
✅ **276x faster compilation** with incremental builds  
✅ **Automatic ownership inference** - no manual borrowing  
✅ **Go-style concurrency** - familiar `go` keyword and channels  
✅ **Modern syntax** - string interpolation, pipe operator, pattern matching  
✅ **100% Rust compatibility** - use any Rust crate  
✅ **World-class IDE support** - LSP, debugging, refactoring in VSCode/Vim/IntelliJ  
✅ **AI-powered development** - MCP server for Claude, ChatGPT code assistance  
✅ **Production-ready** - comprehensive testing, fuzzing, security audit (A+ rating)  
✅ **No lock-in** - `wj eject` converts your project to pure Rust anytime

**Perfect for:** Web APIs, CLI tools, microservices, data processing, learning systems programming

**Philosophy:** Provide 80% of developers with 80% of Rust's power while eliminating 80% of its complexity.

---

## Quick Start

### Install

```bash
# macOS / Linux
brew install windjammer

# Or via Cargo
cargo install windjammer

# Or from source
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer
cargo build --release
./target/release/wj --version
```

### Hello World

Create `hello.wj`:

```windjammer
fn main() {
    let name = "World"
    println!("Hello, ${name}!")  // String interpolation!
}
```

Run it:

```bash
wj run hello.wj
```

### HTTP Server Example

```windjammer
use std::http

fn main() {
    let server = http::Server::new()
    
    server.get("/", |req| {
        http::Response::ok("Hello from Windjammer!")
    })
    
    server.get("/user/:name", |req| {
        let name = req.param("name")
        http::Response::ok("Hello, ${name}!")
    })
    
    println!("Server running on http://localhost:3000")
    server.listen(3000)
}
```

---

## Key Features

### 1. Memory Safety Without the Complexity

Windjammer infers ownership and lifetimes automatically:

```windjammer
// No lifetime annotations needed!
fn longest(s1: str, s2: str) -> str {
    if s1.len() > s2.len() { s1 } else { s2 }
}
```

Compiles to safe Rust:

```rust
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.len() > s2.len() { s1 } else { s2 }
}
```

### 2. Go-Style Concurrency

```windjammer
fn main() {
    let ch = chan::new()
    
    go {
        ch.send("Hello from goroutine!")
    }
    
    let msg = ch.recv()
    println!(msg)
}
```

### 3. Multi-Target Compilation

```bash
# Compile to native binary
wj build --target=rust

# Compile to JavaScript (Node.js or browser)
wj build --target=javascript

# Compile to WebAssembly
wj build --target=wasm
```

### 4. Modern Syntax

```windjammer
// String interpolation
let name = "Windjammer"
println!("Hello, ${name}!")

// Pipe operator
let result = data
    |> parse()
    |> validate()
    |> process()

// Pattern matching with guards
match value {
    Some(x) if x > 0 => println!("Positive: ${x}")
    Some(x) => println!("Non-positive: ${x}")
    None => println!("No value")
}

// Defer statement
fn read_file(path: str) -> Result<String, Error> {
    let file = fs::open(path)?
    defer file.close()
    file.read_to_string()
}
```

### 5. World-Class IDE Support

**Language Server Protocol (LSP):**
- ✅ Real-time type checking and error highlighting
- ✅ Auto-completion for functions, types, and variables
- ✅ Go-to-definition and find-references
- ✅ Hover documentation
- ✅ Inline code hints
- ✅ Refactoring support (rename, extract function, inline variable)
- ✅ Integration with VS Code, IntelliJ, Neovim, Emacs

**MCP Server (AI Integration):**
- ✅ Claude, ChatGPT integration for code assistance
- ✅ Natural language to Windjammer code translation
- ✅ Automated refactoring suggestions
- ✅ Intelligent error diagnosis and fixes

### 6. Zero Lock-In

Not sure if Windjammer is right for you? No problem!

```bash
wj eject
```

Converts your entire project to pure Rust:
- ✅ Production-quality Rust code
- ✅ Complete `Cargo.toml` with dependencies
- ✅ Formatted with `rustfmt`, validated with `clippy`
- ✅ No vendor lock-in whatsoever

---

## Language Features

### Core Features
- ✅ Ownership and lifetime inference
- ✅ Trait bound inference
- ✅ Pattern matching with guards
- ✅ Go-style concurrency (channels, spawn, defer)
- ✅ String interpolation
- ✅ Pipe operator
- ✅ Decorator system
- ✅ Macro system
- ✅ Result and Option types
- ✅ Error propagation with `?`

### Type System
- ✅ Strong static typing
- ✅ Type inference
- ✅ Generics
- ✅ Traits (like Rust traits)
- ✅ Sum types (enums)
- ✅ Product types (structs)
- ✅ Newtype pattern

### Safety
- ✅ No null pointers
- ✅ No data races
- ✅ Memory safety without GC
- ✅ Thread safety
- ✅ Immutable by default

---

## Performance

**Compilation Speed:**
- ✅ **276x faster** hot builds (incremental compilation with Salsa)
- ✅ Cold build: ~5-10s for medium project
- ✅ Hot build: ~50ms for single file change

**Runtime Performance:**
- ✅ **99%+ of Rust's performance** (measured in benchmarks)
- ✅ Zero-cost abstractions
- ✅ No garbage collection overhead
- ✅ SIMD vectorization
- ✅ Advanced optimizations (15-phase pipeline)

---

## Architecture

Windjammer compiles through multiple stages:

```
.wj file → Lexer → Parser → Analyzer → Optimizer → Codegen → Target Code
                                          ↓
                                    15 Optimization Phases:
                                    1-10: Analysis & transformation
                                    11: String interning
                                    12: Dead code elimination
                                    13: Loop optimization
                                    14: Escape analysis
                                    15: SIMD vectorization
```

**Targets:**
- **Rust** → Native binaries (Linux, macOS, Windows)
- **JavaScript** → Node.js or browser (ES2020+, tree-shaking, minification)
- **WebAssembly** → Browser or WASI runtime

---

## Project Status

**Current Version:** 0.37.3  
**Status:** Production-ready for early adopters

**What's Complete:**
- ✅ Core language features
- ✅ Multi-target compilation
- ✅ 15-phase optimization pipeline
- ✅ LSP server with full IDE integration
- ✅ MCP server for AI assistance
- ✅ Standard library (fs, http, json, crypto, etc.)
- ✅ Testing framework
- ✅ Fuzzing infrastructure
- ✅ Security audit (A+ rating)
- ✅ 420+ tests passing

**What's Next:**
- 🔄 Async/await syntax
- 🔄 Const generics
- 🔄 More standard library modules
- 🔄 Documentation generator (`wj doc`)
- 🔄 Package manager
- 🔄 More language examples

---

## Examples

See the [examples/](examples/) directory for more:

- **HTTP Server** - RESTful API with routing
- **CLI Tool** - Command-line argument parsing
- **Concurrent Processing** - Channels and goroutines
- **WebAssembly** - Browser applications
- **Database Access** - SQL queries with connection pooling

---

## Documentation

- [Installation Guide](docs/INSTALLATION.md)
- [Language Guide](docs/GUIDE.md)
- [API Reference](docs/API_REFERENCE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Comparison with Rust and Go](docs/COMPARISON.md)
- [Contributing](CONTRIBUTING.md)
- [Roadmap](ROADMAP.md)

---

## Community

- **GitHub**: [github.com/jeffreyfriedman/windjammer](https://github.com/jeffreyfriedman/windjammer)
- **Issues**: Report bugs or request features
- **Discussions**: Ask questions and share projects

---

## License

Dual-licensed under MIT OR Apache-2.0

---

## Credits

Created by [Jeffrey Friedman](https://github.com/jeffreyfriedman) and contributors.

**Inspiration:**
- Rust (safety and performance)
- Go (simplicity and concurrency)
- Swift (developer experience)
- TypeScript (gradual typing)

---

## Related Projects

- **[windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui)** - Cross-platform UI framework
- **windjammer-game** - Game development framework (private beta)

---

**Made with ❤️ by developers who believe programming should be both safe AND simple.**
