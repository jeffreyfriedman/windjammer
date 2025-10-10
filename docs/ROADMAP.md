# Windjammer Language Roadmap

## Current Status: v0.1 - Core Language Working

✅ **Completed:**
- Lexer, Parser, Analyzer, Code Generator
- Basic types: int, float, bool, string
- Generic types: Vec<T>, Option<T>, Result<T, E>
- Structs and impl blocks
- Pattern matching with guards, OR patterns, tuple patterns
- For loops with ranges
- Closures
- Go-style channel operators (`<-` send/receive)
- Ownership inference
- Self parameters (&self, &mut self)
- Async/await
- Decorators
- Method calls and field access
- Type casts (as)
- References (&, &mut)

---

## Phase 1: Language Enhancements (v0.2)
**Goal:** Make Windjammer significantly easier and more expressive to use
**Timeline:** Next major release

### 1.1 String Interpolation ⭐️
**Priority:** HIGH | **Impact:** HIGH | **Complexity:** LOW

```go
// Before
println("Hello, {}!", name)
println("Sum: {}", x + y)

// After
println("Hello, ${name}!")
println("Sum: ${x + y}")
```

**Implementation:**
- Lexer: Detect `${` inside string literals
- Parser: Parse interpolated expressions
- Codegen: Generate `format!` calls

**Files to modify:**
- `src/lexer.rs` - String literal parsing
- `src/parser.rs` - Expression in strings
- `src/codegen.rs` - format! macro generation

---

### 1.2 Pipe Operator ⭐️⭐️
**Priority:** HIGH | **Impact:** VERY HIGH | **Complexity:** MEDIUM

```go
// Before
let result = parse(validate(read("file.txt")))

// After  
let result = "file.txt"
    |> read
    |> validate
    |> parse
```

**Implementation:**
- Lexer: Add `|>` token
- Parser: Binary operator with special precedence
- Codegen: Rewrite `a |> f` as `f(a)`

**Files to modify:**
- `src/lexer.rs` - Add token
- `src/parser.rs` - Pipe as binary operator
- `src/codegen.rs` - Transform to function call

---

### 1.3 Labeled Arguments
**Priority:** MEDIUM | **Impact:** HIGH | **Complexity:** MEDIUM

```go
fn create_user(name: string, age: int, email: string, active: bool) {
    // ...
}

// Usage with labels
create_user(
    name: "Alice",
    age: 30,
    email: "alice@example.com",
    active: true,
)
```

**Implementation:**
- Parser: Already supports this in decorators, extend to all function calls
- Analyzer: Verify label names match parameters
- Codegen: Generate positional arguments in correct order

**Files to modify:**
- `src/parser.rs` - Extend function call parsing
- `src/analyzer.rs` - Validate labels
- `src/codegen.rs` - Reorder arguments

---

### 1.4 Pattern Matching in Function Parameters
**Priority:** MEDIUM | **Impact:** MEDIUM | **Complexity:** LOW

```go
// Destructuring tuples
fn process((x, y): (int, int)) -> int {
    x + y
}

// Destructuring structs
fn greet(User { name, age }: User) {
    println("${name} is ${age}")
}
```

**Implementation:**
- Parser: Allow patterns instead of just identifiers in parameters
- Codegen: Generate Rust destructuring

**Files to modify:**
- `src/parser.rs` - Parameter parsing
- `src/codegen.rs` - Generate destructuring syntax

---

### 1.5 Trait System with Ergonomic Enhancements ⭐️⭐️⭐️
**Priority:** CRITICAL | **Impact:** VERY HIGH | **Complexity:** HIGH

See `TRAITS_DESIGN.md` for complete design documentation.

```go
// Basic traits
trait Drawable {
    fn draw(&self)
    fn area(&self) -> f64
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("Circle")
    }
    
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}

// Automatic trait bound inference
fn process<T>(item: T) {
    println!("{:?}", item)      // Infers T: Debug
    let copy = item.clone()     // Infers T: Clone
}

// Associated types as generics
trait Iterator<Item> {
    fn next(&mut self) -> Option<Item>
}

// Auto-derive
@auto
struct Point {
    x: int,
    y: int,
}
```

**Implementation Plan:**

**Phase 1 - Core Traits:**
- Parse `trait Name { methods }` definitions
- Parse `impl Trait for Type` blocks
- Basic generic trait bounds `<T: Trait>`
- Codegen for traits → Rust traits

**Phase 2 - Ergonomic Features:**
- ⭐️ **Trait bound inference** from method calls
- ⭐️ **Associated types as generics** (simpler syntax)
- ⭐️ **@auto derive** based on usage patterns
- Ownership inference for trait methods

**Phase 3 - Advanced:**
- Trait aliases (`trait Printable = Display + Debug`)
- Multiple bounds with `+`
- Where clauses
- Default implementations

**Files to modify:**
- `src/lexer.rs` - Add `Trait` keyword
- `src/parser.rs` - Parse trait definitions and impl blocks
- `src/analyzer.rs` - **Trait bound inference engine**
- `src/codegen.rs` - Generate Rust traits

**Benefits:**
- Full Rust trait system power
- Automatic trait bound inference (less boilerplate)
- Cleaner associated type syntax
- Smart auto-derive
- 1:1 mapping to Rust traits

---

## Phase 2: Standard Library (v0.3)
**Goal:** Cover 80% of developer needs without external dependencies
**Timeline:** After Phase 1 stabilizes (including traits)

### Core Modules (Priority Order)

#### 2.1 std/testing - Built-in Test Framework
**Priority:** CRITICAL | **Complexity:** MEDIUM

```go
use std.testing

#[test]
fn test_add() {
    assert_eq!(add(1, 2), 3)
    assert_eq!(add(-5, 5), 0)
}

#[test]
async fn test_fetch() {
    let result = fetch("https://api.example.com").await
    assert!(result.is_ok())
}
```

**Implementation:**
- Create `std/testing/mod.wj`
- Transpile to Rust's `#[test]` attribute
- Support async tests
- Generate test runner

---

#### 2.2 std/http - HTTP Client & Server
**Priority:** CRITICAL | **Complexity:** HIGH

```go
use std.http

// Client
async fn get_data() -> Result<string, Error> {
    let response = http.get("https://api.example.com").await?
    let data = response.json()?
    Ok(data)
}

// Server
#[tokio::main]
async fn main() {
    let app = http.Router.new()
        .get("/", || "Hello, World!")
        .get("/users/:id", |id: int| format!("User {}", id))
    
    http.serve("127.0.0.1:3000", app).await
}
```

**Wraps:** reqwest (client), axum (server)

---

#### 2.3 std/json - JSON Encoding/Decoding
**Priority:** CRITICAL | **Complexity:** LOW

```go
use std.json

struct User {
    name: string,
    age: int,
}

let user = User { name: "Alice", age: 30 }
let json_str = json.encode(user)?
let decoded: User = json.decode(json_str)?
```

**Wraps:** serde_json

---

#### 2.4 std/fs - File System Operations
**Priority:** HIGH | **Complexity:** LOW

```go
use std.fs

// Read/Write
let content = fs.read_to_string("data.txt")?
fs.write("output.txt", "Hello, World!")?

// Directory operations
fs.create_dir_all("path/to/dir")?
let entries = fs.read_dir(".")?
```

**Wraps:** std::fs

---

#### 2.5 std/fmt - Formatting & Logging
**Priority:** HIGH | **Complexity:** LOW

```go
use std.fmt

fmt.print("Hello")
fmt.println("World")
fmt.debug(user)  // Pretty-print debug

// Logging
fmt.info("Server started")
fmt.error("Connection failed: ${err}")
fmt.warn("Deprecated API used")
```

**Wraps:** println!, log crate

---

#### 2.6 std/cli - Argument Parsing
**Priority:** MEDIUM | **Complexity:** LOW

```go
use std.cli

@command(name: "mytool", version: "1.0")
struct Args {
    @arg(short: 'o', long: "output")
    output: string,
    
    @flag(short: 'v', long: "verbose")
    verbose: bool,
    
    files: Vec<string>,
}

fn main() {
    let args = cli.parse<Args>()
    println("Output: ${args.output}")
}
```

**Wraps:** clap

---

#### 2.7 std/time - Time & Duration
**Priority:** MEDIUM | **Complexity:** LOW

```go
use std.time

let now = time.now()
let duration = time.Duration.from_secs(30)
time.sleep(duration).await

let formatted = now.format("%Y-%m-%d %H:%M:%S")
```

**Wraps:** std::time, chrono

---

#### 2.8 std/crypto - Cryptographic Functions
**Priority:** MEDIUM | **Complexity:** MEDIUM

```go
use std.crypto

// Hashing
let hash = crypto.sha256("password")
let md5 = crypto.md5(data)

// Random
let random_bytes = crypto.random_bytes(32)
let uuid = crypto.uuid()
```

**Wraps:** sha2, md5, uuid, rand

---

### Later Modules (v0.4+)

- `std/db` - Database connections (postgres, mysql, sqlite)
- `std/encoding` - base64, hex, url encoding
- `std/net` - TCP, UDP, WebSocket
- `std/regex` - Regular expressions  
- `std/template` - HTML/text templating
- `std/os` - OS interface (env, args, processes)

---

## Phase 3: Doctests (v0.4)
**Priority:** MEDIUM | **Complexity:** MEDIUM

### Rust-Style Documentation Tests

```go
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// assert_eq!(add(1, 2), 3)
/// assert_eq!(add(-5, 5), 0)
/// ```
fn add(a: int, b: int) -> int {
    a + b
}
```

**Implementation:**
- Parse `///` doc comments
- Extract code blocks
- Generate test functions
- Run as part of `wj test`

**Files to modify:**
- `src/parser.rs` - Parse doc comments
- New: `src/doctest.rs` - Extract and generate tests
- `src/main.rs` - Add `test` command

---

## Phase 4: Advanced Features (v0.5+)

### 4.1 Macro System
```go
macro_rules! vec {
    ($($x:expr),*) => {
        Vec::from([$($x),*])
    }
}
```

### 4.2 Trait-like Interfaces
```go
trait Drawable {
    fn draw(&self)
}

impl Drawable for Circle {
    fn draw(&self) {
        println("Drawing circle")
    }
}
```

### 4.3 Multiple Function Clauses (Elixir-style)
```go
fn handle(Ok(value): Result<int, Error>) -> int {
    value
}

fn handle(Err(_): Result<int, Error>) -> int {
    0
}
```

### 4.4 Language Server Enhancements
- Go-to-definition
- Find references
- Rename refactoring
- Code actions (quick fixes)
- Inlay hints for ownership

---

## Phase 5: Ecosystem & Tooling (v1.0)

### 5.1 Package Manager
```bash
windjammer add github.com/user/package
windjammer update
windjammer publish
```

### 5.2 Build System
```toml
# Windjammer.toml
[package]
name = "myapp"
version = "0.1.0"

[dependencies]
http = "0.1"
json = "0.1"
```

### 5.3 Formatter
```bash
wj fmt
```

### 5.4 Linter
```bash
wj lint
```

### 5.5 Documentation Generator
```bash
windjammer doc
# Generates beautiful docs like docs.rs
```

---

## Success Metrics

### v0.2 Goals:
- ✅ String interpolation working
- ✅ Pipe operator working
- ✅ Can write real programs without fighting syntax
- ✅ All examples compile and run

### v0.3 Goals:
- ✅ std/http, std/json, std/fs, std/testing working
- ✅ Can build a full web API without external crates
- ✅ Documentation for all stdlib modules

### v0.4 Goals:
- ✅ Doctests running automatically
- ✅ Test coverage > 80%
- ✅ All stdlib docs have working examples

### v1.0 Goals:
- ✅ Production-ready compiler
- ✅ Comprehensive stdlib
- ✅ Great documentation
- ✅ Active community
- ✅ Package ecosystem starting

---

## Development Principles

1. **Pragmatic over Pure**: Choose practical solutions over theoretical perfection
2. **Rust Compatibility**: Always transpile to idiomatic Rust
3. **Developer Experience**: Prioritize ease of use and clear error messages
4. **Stability**: Don't break existing code
5. **Documentation**: Every feature must have examples
6. **Testing**: Comprehensive tests for all features
7. **Performance**: Generated Rust should be fast
8. **Simplicity**: One obvious way to do things

---

## Community Feedback

After each phase, gather feedback on:
- What's confusing?
- What's missing?
- What should be removed?
- What should be prioritized?

---

## Release Schedule

- **v0.1** (Current): Core language working
- **v0.2** (Q1 2024): Language enhancements
- **v0.3** (Q2 2024): Standard library
- **v0.4** (Q3 2024): Doctests & tooling
- **v0.5** (Q4 2024): Advanced features
- **v1.0** (Q1 2025): Production ready

---

## Next Steps

Immediate priorities:
1. ✅ Fix remaining parser issues with examples
2. ⏳ Implement string interpolation
3. ⏳ Implement pipe operator
4. ⏳ Create std/ directory structure
5. ⏳ Implement std/testing
6. ⏳ Write comprehensive GUIDE.md updates

**Let's build the language developers will love! 🚀**

