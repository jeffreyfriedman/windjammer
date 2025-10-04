# Windjammer

A simple, high-level language that transpiles to Rust—combining Go's ergonomics, Ruby's expressiveness, and Rust's safety and performance.

> **🎯 The 80/20 Language**: 80% of Rust's power with 20% of the complexity  
> **📊 [Read the detailed comparison: Windjammer vs Rust vs Go](docs/COMPARISON.md)**

## Philosophy

**Write Simple, Run Fast** - Ergonomic syntax that transpiles to safe, efficient Rust code.

Windjammer takes the best ideas from modern languages:
- **Go**: Simple concurrency (`go` keyword, channels)
- **Ruby**: String interpolation, expressive syntax
- **Elixir**: Pipe operator for data transformations
- **Python**: Clean decorators
- **Rust**: Safety, performance, powerful type system

**The 80/20 Rule**: Windjammer provides 80% of Rust's power (memory safety, zero-cost abstractions, performance) while eliminating 80% of the complexity (manual lifetime annotations, explicit borrowing, verbose syntax).

## Key Features

### 🎯 Automatic Ownership Inference
No need to think about borrowing in most cases - the compiler figures it out.

### 🎨 Python-style Decorators
Clean, intuitive syntax for cross-cutting concerns like routing, caching, and logging.

### ⚡ Go-style Concurrency
Familiar `go` keyword and channels with **Go's `<-` operator**, but with Rust's safety guarantees.

### 🛡️ Rust's Safety & Performance
Zero-cost abstractions, memory safety, and blazing speed.

### 🚀 Modern Language Features
- **Generic types**: `Vec<T>`, `Option<T>`, `Result<T, E>`
- **Pattern matching** with guards and tuple patterns
- **Closures**: `|x| x * 2`
- **Range expressions**: `0..10` and `0..=10`
- **Struct shorthand**: `User { name, age }`
- **Method syntax**: `impl` blocks with `&self` and `&mut self`
- **String interpolation**: `"Hello, ${name}!"` ✨
- **Pipe operator**: `value |> func1 |> func2` ✨
- **Ternary operator**: `condition ? true_val : false_val` ✨
- **Labeled arguments**: `create_user(name: "Alice", age: 30)` ✨
- **Pattern matching in function parameters**: `fn process((x, y): (int, int))` ✨
- **Smart @auto derive**: Zero-config trait inference (`@auto`) ✨
- **Trait system**: Full trait definitions and implementations ✨

## Ownership Inference Strategy

The key innovation is **automatic ownership inference** with these rules:

### 1. Smart Defaults
- **Primitive types** (int, float, bool): Always copied (implement Copy trait)
- **String literals**: Owned by default
- **Struct fields**: Owned by default unless marked otherwise
- **Function parameters**: Borrowed by default (immutable `&T`)
- **Function returns**: Owned by default

### 2. Mutation Detection
```windjammer
// Immutable borrow inferred
fn print_length(s: string) {
    println!("{}", s.len())
}

// Mutable borrow inferred from assignment
fn increment(x: int) {
    x = x + 1  // Assignment detected → infers &mut
}

// Mutable borrow inferred from method call
fn append_text(s: string) {
    s.push_str("!")  // Mutation detected → infers &mut
}

// Ownership transfer inferred from return
fn take_ownership(s: string) -> string {
    s  // Returned → infers owned parameter
}
```

### 3. Escape Analysis
Like Go, we analyze if values escape the function:
- Values that don't escape → borrow
- Values that escape (returned, stored) → owned
- Ambiguous cases → owned (safe default)

### 4. Explicit Annotations (Rare)
When inference isn't enough, use Rust-style annotations:
```go
fn process(s: &string) -> string     // Explicit borrow
fn modify(s: &mut string)            // Explicit mutable borrow
fn take(s: string) -> string         // Explicit ownership (inferred by default for returns)
```

### 5. Shared Ownership
When multiple owners needed:
```go
// Automatic Rc/Arc when:
// - Value assigned to multiple variables in different scopes
// - Value stored in multiple struct fields
// - Cross-thread sharing detected → Arc

let data = shared("expensive data")  // Explicit Rc/Arc
```

## Decorators

Use `@` for decorators (Python/TypeScript style):

```go
@route("/api/users")
@auth_required
fn get_users() -> Result<Vec<User>, Error> {
    // ... handler logic
}

@timing
@cache(ttl: 60)
fn expensive_calculation(n: int) -> int {
    // ... complex logic
}
```

Built-in decorators:
- `@timing` - Measure execution time
- `@cache` - Memoize function results
- `@route(path)` - HTTP routing (with web frameworks)
- `@get`, `@post`, `@put`, `@delete` - HTTP method routing

## Concurrency

Go-style syntax with Rust safety, including **Go-style channel operators**:

```go
use std.sync.mpsc

fn main() {
    let (tx, rx) = mpsc.channel()
    
    // Spawn a goroutine (maps to tokio::spawn or std::thread)
    go {
        tx <- "Hello from thread"  // Go-style send!
    }
    
    let msg = <-rx  // Go-style receive!
    println(msg)
}

// Traditional Rust syntax also works:
fn alternative_example() {
    let (tx, rx) = mpsc.channel()
    go {
        tx.send("Hello").unwrap()
    }
    let msg = rx.recv().unwrap()
}

// Async/await style also supported
async fn fetch_data(url: string) -> Result<string, Error> {
    let response = http.get(url).await?
    Ok(response.text().await?)
}
```

**Channel Operators:**
- `channel <- value` — Send to channel (transpiles to `channel.send(value)`)
- `<-channel` — Receive from channel (transpiles to `channel.recv()`)

## Syntax Examples

### Variables
```go
let x = 42              // Transpiles to: let x = 42;
let mut y = 10          // Transpiles to: let mut y = 10;
```

### Functions
```go
fn add(a: int, b: int) -> int {
    a + b
}
// Transpiles to:
// fn add(a: i32, b: i32) -> i32 { a + b }
```

### Structs and Impl Blocks
```go
struct User {
    name: string,
    age: int,
}

impl User {
    // Associated function (like "static" in other languages)
    fn new(name: string, age: int) -> User {
        User { name, age }  // Shorthand syntax supported!
    }
    
    // Method with &self
    fn greet(&self) {
        println("Hello, I'm {}", self.name)
    }
    
    // Method with &mut self
    fn birthday(&mut self) {
        self.age += 1
    }
}

fn main() {
    let mut user = User.new("Alice", 30)
    user.greet()
    user.birthday()
    println("{} is now {}", user.name, user.age)
}
```

**Struct Features:**
- **Shorthand field initialization**: `User { name, age }` when variables match field names
- **Self parameters**: `&self`, `&mut self`, or `self` (owned)
- **Methods and associated functions** via `impl` blocks

### Error Handling
```go
use std.fs
use std.io

fn read_config(path: string) -> Result<string, Error> {
    let contents = fs.read_to_string(path)?
    Ok(contents)
}
```

### Generic Types
```go
// Vec<T> - Dynamic arrays
let numbers: Vec<int> = vec![1, 2, 3, 4, 5]
let names: Vec<string> = Vec.new()

// Option<T> - Optional values
fn find_user(id: int) -> Option<User> {
    if id == 1 {
        Some(User.new("Alice", 30))
    } else {
        None
    }
}

// Result<T, E> - Error handling
fn read_file(path: string) -> Result<string, Error> {
    let contents = fs.read_to_string(path)?
    Ok(contents)
}
```

### Pattern Matching
```go
// Match as an expression
let value = Some(42)
let result = match value {
    Some(n) => n * 2,
    None => 0,
}

// Match with guards
match (x, y) {
    (0, 0) => println("Origin"),
    (x, 0) if x > 0 => println("Positive X axis"),
    (0, y) if y > 0 => println("Positive Y axis"),
    _ => println("Somewhere else"),
}

// Match with tuple patterns
match (cell, live_neighbors) {
    (true, x) if x < 2 => false,    // Underpopulation
    (true, 2) | (true, 3) => true,  // Survival
    (true, x) if x > 3 => false,    // Overpopulation
    (false, 3) => true,              // Reproduction
    (otherwise, _) => otherwise,     // No change
}
```

### Closures and Iterators
```go
// Closures with Go-style syntax
let double = |x| x * 2
let add = |a, b| a + b

// Iterator methods
let numbers = vec![1, 2, 3, 4, 5]
let doubled = numbers.iter().map(|n| n * 2).collect()
let evens = numbers.iter().filter(|n| n % 2 == 0).collect()

// Range expressions
for i in 0..10 {        // Exclusive range: 0 to 9
    println("{}", i)
}

for i in 0..=10 {       // Inclusive range: 0 to 10
    println("{}", i)
}
```

### References and Borrowing
```go
// Create references
let x = 42
let ref_x = &x
let mut_ref_x = &mut x

// Function parameters can be explicit or inferred
fn read_value(x: &int) -> int {   // Explicit borrow
    *x
}

fn modify_value(x: &mut int) {    // Explicit mutable borrow
    *x += 1
}

fn consume_value(x: int) {         // Explicit ownership
    println("{}", x)
}

// Or let the compiler infer!
fn inferred_borrow(x: int) {       // Auto-inferred as &int
    println("{}", x)
}
```

### String Interpolation ✨
```go
let name = "Alice"
let age = 30
let score = 95.5

// Clean, readable string formatting
println!("Hello, ${name}!")
println!("${name} is ${age} years old")
println!("Score: ${score * 100.0}%")

// Works with any expression
let message = "Result: ${calculate(x, y) + 10}"
```

### Pipe Operator ✨
```go
fn double(x: int) -> int { x * 2 }
fn add_ten(x: int) -> int { x + 10 }
fn to_string(x: int) -> string { format!("{}", x) }

// Functional composition made readable
let result = 5 
    |> double 
    |> add_ten 
    |> double
    |> to_string

// Equivalent to: to_string(double(add_ten(double(5))))
// Result: "40"
```

### Labeled Arguments ✨
```go
fn create_user(name: string, age: int, email: string) {
    println!("User: ${name}, Age: ${age}")
}

// Traditional positional arguments
create_user("Alice", 30, "alice@example.com")

// Labeled arguments for clarity (especially with many parameters)
create_user(
    name: "Bob",
    age: 25,
    email: "bob@example.com"
)

// Mix positional and labeled
create_user("Charlie", age: 35, email: "charlie@example.com")

// Great for functions with many parameters or similar types
fn configure_server(
    host: string,
    port: int,
    timeout: int,
    max_connections: int,
    enable_logging: bool
) {
    // ...
}

// Much clearer than: configure_server("localhost", 8080, 30, 100, true)
configure_server(
    host: "localhost",
    port: 8080,
    timeout: 30,
    max_connections: 100,
    enable_logging: true
)
```

### Pattern Matching in Function Parameters ✨
```go
// Instead of manually destructuring...
fn distance_old(point: (int, int)) -> float {
    let (x, y) = point  // Manual destructuring
    sqrt(x * x + y * y)
}

// Destructure directly in the parameter!
fn distance((x, y): (int, int)) -> float {
    sqrt(x * x + y * y)  // x and y ready to use!
}

// Real-world example: API response handler
fn handle_response((status, body): (int, string)) {
    if status == 200 {
        println!("Success: ${body}")
    } else {
        println!("Error ${status}: ${body}")
    }
}

let response = (200, "Data loaded")
handle_response(response)

// Think of it as: PATTERN : TYPE
// (x, y) ← pattern | (int, int) ← type
```

### @auto Derive ✨
```go
// Automatic trait derivation - no manual impl needed!
@auto(Debug, Clone, Copy)
struct Point {
    x: int,
    y: int,
}

@auto(Debug, Clone)
struct User {
    name: string,
    age: int,
}

let p1 = Point { x: 10, y: 20 }
let p2 = p1  // Copy works automatically
println!("{:?}", p1)  // Debug works automatically

// Equivalent to Rust's #[derive(Debug, Clone, Copy)]
// But with cleaner, more intuitive syntax!
```

### Trait System ✨
```go
// Define traits
trait Drawable {
    fn draw(&self)
    fn area(&self) -> f64
}

// Implement traits
struct Circle {
    radius: f64,
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("Drawing circle with radius ${self.radius}")
    }
    
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}

// Generic traits with associated types
trait Iterator<Item> {
    fn next(&mut self) -> Option<Item>
}

impl Iterator<int> for Counter {
    fn next(&mut self) -> Option<int> {
        // ...
    }
}
```

## Type Mappings

| Windjammer | Rust |
|---------|------|
| int | i64 |
| int32 | i32 |
| uint | u64 |
| float | f64 |
| bool | bool |
| string | String (owned) or &str (borrowed) |
| Vec\<T\> | Vec\<T\> |
| Option\<T\> | Option\<T\> |
| Result\<T,E\> | Result\<T,E\> |
| &T | &T (immutable reference) |
| &mut T | &mut T (mutable reference) |

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/windjammer-lang/windjammer
cd windjammer

# Build the compiler
cargo build --release

# Install the language server
cd crates/windjammer-lsp
cargo install --path .

# Install VSCode extension
cd ../../editors/vscode
npm install
npm run compile
code --install-extension windjammer-*.vsix
```

### Your First Program

Create `hello.wj`:

```go
use std.println

fn main() {
    println("Hello, Windjammer!")
}
```

Transpile and run:

```bash
windjammer build hello.wj
cd output
cargo run
```

## Examples

See the `examples/` directory for complete working examples:

1. **[HTTP Server](examples/http_server/)** - RESTful API with decorators and async
2. **[CLI Tool](examples/cli_tool/)** - Concurrent file processor with channels
3. **[WASM Game](examples/wasm_game/)** - Conway's Game of Life compiled to WebAssembly

## Development Tools

### Language Server

Windjammer includes a high-performance language server built with **Salsa** for incremental compilation:

- ⚡ **Fast**: Only recomputes what changed
- 🔍 **IntelliSense**: Auto-completion, hover info, diagnostics
- 🎯 **Ownership Hints**: See inferred borrowing inline
- 📝 **Instant Feedback**: Real-time error checking

Install: `cargo install --path crates/windjammer-lsp`

### VSCode Extension

Features:
- Syntax highlighting
- Auto-completion
- Error diagnostics
- Code navigation
- Ownership inference hints

Install from `editors/vscode/` or search "Windjammer" in the marketplace (coming soon).

## Build & Run

```bash
# Transpile .wj files to .rs
windjammer build

# Or specify files
windjammer build src/main.wj

# Check for errors without generating code
windjammer check

# Run the generated Rust code
cd output
cargo run
```

## When to Use Windjammer

### ✅ Choose Windjammer For

- **Web APIs & Services** - Built-in HTTP, JSON, ergonomic syntax
- **CLI Tools** - Fast development with system-level performance  
- **Data Processing** - Pipe operator for clean transformations
- **Microservices** - Fast, safe, memory-efficient
- **Learning Systems Programming** - Gentler curve than Rust
- **Teams Transitioning from Go** - Familiar syntax, better performance
- **80% of Use Cases** - Most applications don't need Rust's full complexity

### ⚠️ Consider Rust Instead For

- **Operating Systems** - Need maximum low-level control
- **Embedded Systems** - Need `no_std` and precise memory management
- **Game Engines** - Need every manual optimization
- **The Critical 20%** - When you truly need Rust's full power

### ⚠️ Consider Go Instead For

- **Dead-Simple Services** - No performance requirements
- **Rapid Prototypes** - Speed over safety
- **Teams Unfamiliar with Systems** - Easiest learning curve

**📊 See [COMPARISON.md](docs/COMPARISON.md) for detailed analysis**

---

## 📘 Documentation

**For Users**:
- 📖 **[GUIDE.md](docs/GUIDE.md)** - Complete developer guide (Rust book style)
- 🔄 **[COMPARISON.md](docs/COMPARISON.md)** - Windjammer vs Rust vs Go (honest tradeoffs)
- 🎯 **[README.md](README.md)** - This file (quick start and overview)

**For Contributors**:
- 🚀 **[PROGRESS.md](docs/PROGRESS.md)** - Current status and next steps
- 🗺️ **[ROADMAP.md](docs/ROADMAP.md)** - Development phases and timeline
- 🎨 **[Traits Design](docs/design/traits.md)** - Ergonomic trait system design
- 🔧 **[Auto-Reference Design](docs/design/auto-reference.md)** - Automatic reference insertion
- 📍 **[Error Mapping Design](docs/design/error-mapping.md)** - Rust→Windjammer error translation

**Standard Library**:
- 📚 **[std/README.md](std/README.md)** - Philosophy and architecture
- 📦 **std/*/API.md** - Module specifications (fs, http, json, testing)

---

## Rust Interoperability

**✅ YES: 100% Rust Crate Compatibility!**

Windjammer transpiles to Rust, giving you:

- ✅ **ALL Rust crates work** - Tokio, Serde, Actix, Reqwest, etc.
- ✅ **No FFI needed** - It IS Rust under the hood
- ✅ **Same performance** - Zero overhead translation
- ✅ **Mix .wj and .rs files** - Use both in same project
- ✅ **Call Rust from Windjammer** - And vice versa

**Example:**
```windjammer
// Your Windjammer code
use serde.json
use tokio.time

@auto(Serialize, Deserialize)
struct Config {
    timeout: int,
}

// Uses Rust crates directly!
async fn load_config() -> Result<Config, Error> {
    let text = fs.read_to_string("config.json")?
    let config = serde_json::from_str(&text)?
    Ok(config)
}
```

**Transpiles to idiomatic Rust:**
```rust
use serde::{Serialize, Deserialize};
use tokio::time;

#[derive(Serialize, Deserialize)]
struct Config {
    timeout: i64,
}

async fn load_config() -> Result<Config, Error> {
    let text = std::fs::read_to_string("config.json")?;
    let config = serde_json::from_str(&text)?;
    Ok(config)
}
```

---

## 📄 License

Windjammer is dual-licensed under either:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

This means you can choose either license when using Windjammer.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Windjammer by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

