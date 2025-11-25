# Getting Started with Windjammer

**Welcome to Windjammer!** This tutorial will get you up and running in 15 minutes.

---

## What is Windjammer?

Windjammer is a **systems programming language** that gives you:
- ✅ **80% of Rust's power** with **20% of Rust's complexity**
- ✅ **Memory safety** without garbage collection
- ✅ **Performance** matching Rust (98.7% measured)
- ✅ **Simple syntax** inspired by Rust, Python, and Go
- ✅ **100% Rust crate compatibility** (transpiles to Rust)

**Perfect for**: Web APIs, CLI tools, system utilities, microservices

---

## Installation

### Prerequisites
- Rust 1.70+ (Windjammer transpiles to Rust)
- Git

### Install Windjammer CLI

```bash
cargo install windjammer
```

Or build from source:

```bash
git clone https://github.com/windjammer-lang/windjammer
cd windjammer
cargo build --release
cargo install --path .
```

### Verify Installation

```bash
wj --version
# Output: windjammer 0.23.0
```

---

## Your First Program

### Hello World

Create `hello.wj`:

```windjammer
fn main() {
    println!("Hello, Windjammer!")
}
```

**Run it:**

```bash
wj run hello.wj
```

**That's it!** No project setup needed for simple scripts.

---

## Your First Project

### Create a New Project

```bash
wj new my_app
cd my_app
```

This creates:
```
my_app/
├── wj.toml          # Project configuration
├── src/
│   └── main.wj      # Your code
└── Cargo.toml       # Generated Rust config (auto-managed)
```

### Project Structure

**`wj.toml`** - Windjammer's native config:
```toml
[package]
name = "my_app"
version = "0.1.0"

[compiler]
defer_drop = true
defer_drop_threshold = 1024

[dependencies]
# Windjammer dependencies here
```

**`src/main.wj`** - Your code:
```windjammer
fn main() {
    println!("Welcome to my app!")
}
```

### Build and Run

```bash
wj run              # Run your app
wj build            # Build (generates Rust, compiles)
wj test             # Run tests
wj fmt              # Format code
wj lint             # Lint code (uses clippy)
```

---

## Language Basics

### Variables

```windjammer
// Immutable by default (like Rust)
let x = 42
let name = "Alice"

// Mutable when needed
let mut count = 0
count += 1

// Type inference (but explicit types allowed)
let age: int = 30
let pi: float = 3.14
```

### Functions

```windjammer
// Simple function
fn greet(name: string) {
    println!("Hello, ${name}!")
}

// With return type
fn add(a: int, b: int) -> int {
    a + b  // No 'return' needed for last expression
}

// With explicit return
fn is_even(n: int) -> bool {
    return n % 2 == 0
}
```

### String Interpolation

```windjammer
let name = "Bob"
let age = 25

// Built-in interpolation (no format! macro needed)
println!("${name} is ${age} years old")
println!("Next year: ${age + 1}")
```

### Control Flow

```windjammer
// If/else
if age >= 18 {
    println!("Adult")
} else {
    println!("Minor")
}

// Match (pattern matching)
match status {
    "active" => println!("Running"),
    "idle" => println!("Waiting"),
    _ => println!("Unknown"),
}

// Match with values
let message = match count {
    0 => "none",
    1 => "one",
    _ => "many",
}

// Loops
for i in 0..10 {
    println!("${i}")
}

let mut x = 0
while x < 5 {
    x += 1
}
```

### Collections

```windjammer
// Vectors
let numbers = vec![1, 2, 3, 4, 5]
numbers.push(6)

// HashMap
use std::collections.HashMap

let mut scores = HashMap::new()
scores.insert("Alice", 100)
scores.insert("Bob", 85)

// Iteration
for num in numbers {
    println!("${num}")
}

for (name, score) in scores {
    println!("${name}: ${score}")
}
```

### Structs

```windjammer
// Define a struct
@derive(Debug, Clone)]
struct User {
    name: string,
    age: int,
    email: string,
}

// Create an instance
let user = User {
    name: "Alice",
    age: 30,
    email: "alice@example.com",
}

// Access fields
println!("${user.name} is ${user.age}")

// Methods
impl User {
    pub fn new(name: string, age: int, email: string) -> Self {
        User { name, age, email }
    }
    
    pub fn greet(self) {
        println!("Hi, I'm ${self.name}!")
    }
}

let user = User::new("Bob", 25, "bob@example.com")
user.greet()
```

### Error Handling

```windjammer
// Result type (like Rust)
fn divide(a: int, b: int) -> Result<int, string> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Using ? operator
fn calculate() -> Result<int, string> {
    let result = divide(10, 2)?
    Ok(result * 2)
}

// Pattern matching on Result
match divide(10, 0) {
    Ok(val) => println!("Result: ${val}"),
    Err(e) => println!("Error: ${e}"),
}
```

---

## Using the Standard Library

Windjammer has a **comprehensive standard library** with proper abstractions:

### File I/O

```windjammer
use std::fs

// Read a file
let contents = fs::read_to_string("data.txt")?

// Write a file
fs::write("output.txt", "Hello, world!")?

// Check if file exists
if fs.exists("config.json") {
    println!("Config found!")
}
```

### JSON

```windjammer
use std::json

@derive(Serialize, Deserialize)]
struct Config {
    host: string,
    port: int,
}

// Serialize
let config = Config { host: "localhost", port: 8080 }
let json = json::stringify(&config)?

// Deserialize
let config: Config = json::parse(json_string)?
```

### HTTP Client

```windjammer
use std::http

@async
fn fetch_data() -> Result<string, Error> {
    let response = http::get("https://api.example.com/data").await?
    Ok(response.text().await?)
}
```

### HTTP Server

```windjammer
use std::http

@async
fn main() {
    http.serve("127.0.0.1:8080", |req| {
        if http.path(req) == "/" {
            http.json_response(200, { "message": "Hello!" })
        } else {
            http.json_response(404, { "error": "Not found" })
        }
    }).await
}
```

---

## Key Differences from Rust

### 1. **No Manual Lifetime Annotations**

**Rust:**
```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

**Windjammer:**
```windjammer
fn longest(x: string, y: string) -> string {
    if x.len() > y.len() { x } else { y }
}
// Compiler infers ownership automatically!
```

### 2. **Automatic Ownership Inference**

**Rust:**
```rust
let x = String::from("hello");
takes_ownership(x);       // Explicit move
// x is invalid here!

let y = String::from("world");
borrows(&y);              // Explicit borrow
// y is still valid
```

**Windjammer:**
```windjammer
let x = "hello"
takes_ownership(x)  // Compiler decides to move
// x is handled correctly

let y = "world"
borrows(y)  // Compiler decides to borrow
// y is still valid
```

### 3. **String Interpolation Built-in**

**Rust:**
```rust
format!("Hello, {}! You are {} years old.", name, age)
```

**Windjammer:**
```windjammer
"Hello, ${name}! You are ${age} years old."
```

### 4. **Simplified Decorators**

**Rust:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User { ... }
```

**Windjammer:**
```windjammer
@derive(Debug, Clone, Serialize, Deserialize)]
struct User { ... }
```

---

## Best Practices

### 1. **Let the Compiler Help You**

Windjammer's ownership inference is smart. Trust it:

```windjammer
// ✅ Good - let compiler decide
fn process(data: Vec<int>) {
    for item in data {
        println!("${item}")
    }
}

// ❌ Don't overthink ownership
// Just write what you mean!
```

### 2. **Use the Standard Library**

Don't import crates directly when stdlib has it:

```windjammer
// ✅ Good - uses stdlib
use std::http
use std::json
use std::db

// ❌ Avoid - crate leakage
use axum::Router
use serde_json::Value
```

### 3. **Embrace Pattern Matching**

```windjammer
// ✅ Good - clear and safe
match result {
    Ok(val) => process(val),
    Err(e) => log.error("Failed: ${e}"),
}

// ❌ Avoid - can panic
let val = result.unwrap()
```

### 4. **Use Meaningful Names**

```windjammer
// ✅ Good
let user_count = users.len()
let is_valid = validate_input(data)

// ❌ Avoid
let n = users.len()
let x = validate_input(data)
```

---

## Next Steps

**Tutorials**:
1. ✅ **Getting Started** (You are here!)
2. [Building a CLI Tool](./02_CLI_TOOL.md) - Create wjfind from scratch
3. [Building a Web API](./03_WEB_API.md) - Create a REST API
4. [Building a WebSocket Server](./04_WEBSOCKET.md) - Real-time chat

**Documentation**:
- [Language Guide](../GUIDE.md) - Complete language reference
- [Standard Library](../stdlib/README.md) - All stdlib modules
- [Comparison](../COMPARISON.md) - Windjammer vs Rust vs Go
- [Best Practices](../BEST_PRACTICES.md) - Production tips

**Examples**:
- [TaskFlow API](../../examples/taskflow/) - Full REST API
- [wjfind](../../examples/wjfind/) - File search CLI tool
- [wschat](../../examples/wschat/) - WebSocket chat server

---

## Getting Help

- **Discord**: [Join our community](https://discord.gg/windjammer)
- **GitHub**: [Issues and discussions](https://github.com/windjammer-lang/windjammer)
- **Docs**: [Full documentation](https://windjammer-lang.org/docs)

---

## Quick Reference

### Commands

```bash
wj new <name>        # Create new project
wj run               # Run your app
wj build             # Build release binary
wj test              # Run tests
wj fmt               # Format code
wj lint              # Lint code
wj update            # Update Windjammer CLI
```

### Common Types

```windjammer
int, float, bool, string
Vec<T>, HashMap<K, V>, HashSet<T>
Option<T>, Result<T, E>
```

### Stdlib Modules

```windjammer
std.fs       // File system
std.http     // HTTP client + server
std.json     // JSON serialization
std.db       // Database
std.log      // Logging
std.time     // Time operations
std.crypto   // Cryptography
std.regex    // Regular expressions
std.cli      // CLI argument parsing
std.thread   // Threading + parallel
```

---

**Welcome to Windjammer! Start building today! 🚀**

