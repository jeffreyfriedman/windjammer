# The Windjammer Programming Language Guide

**Learn Windjammer** - A comprehensive guide for developers

Welcome to Windjammer! This guide will take you from zero to hero, teaching you how to write Go-like code that transpiles to safe, fast Rust.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Syntax](#basic-syntax)
3. [Functions and Control Flow](#functions-and-control-flow)
4. [Structs and Methods](#structs-and-methods)
5. [Enums and Pattern Matching](#enums-and-pattern-matching)
6. [Ownership and Borrowing](#ownership-and-borrowing)
7. [Generic Types](#generic-types)
8. [Traits](#traits)
   - [Basic Trait Definitions](#basic-trait-definitions)
   - [Trait Bounds](#trait-bounds)
   - [Where Clauses](#where-clauses)
   - [Associated Types](#associated-types)
   - [Trait Objects](#trait-objects)
   - [Supertraits](#supertraits)
   - [Generic Traits](#generic-traits)
9. [String Interpolation](#string-interpolation)
10. [Pipe Operator](#pipe-operator)
11. [Labeled Arguments](#labeled-arguments)
12. [Character Literals](#character-literals)
13. [Concurrency](#concurrency)
14. [Error Handling](#error-handling)
15. [Decorators and Auto-Derive](#decorators-and-auto-derive)
16. [Advanced Topics](#advanced-topics)

---

## Getting Started

### Installation

First, clone and build the Windjammer compiler:

```bash
git clone https://github.com/yourusername/windjammer
cd windjammer
cargo build --release
```

Add the compiler to your PATH:

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$PATH:/path/to/windjammer/target/release"
```

### Hello, World!

Create a file called `hello.wj`:

```windjammer
fn main() {
    println!("Hello, Windjammer!")
}
```

Compile and run:

```bash
wj build --path hello.wj
cd output
cargo run
```

You should see:
```
Hello, Windjammer!
```

**What just happened?**
1. Windjammer transpiled your `.wj` file to Rust (`.rs`)
2. Created a `Cargo.toml` with necessary dependencies
3. You then used Cargo to compile and run the Rust code

---

## Basic Syntax

### Variables and Constants

Windjammer has three kinds of variable declarations:

```windjammer
// Immutable variable (default)
let x = 42
let name = "Alice"

// Mutable variable
let mut counter = 0
counter += 1

// Constant (compile-time constant)
const MAX_SIZE: int = 100

// Static variable (runtime constant)
static GLOBAL_COUNT: int = 0
static mut MUTABLE_GLOBAL: int = 0  // Use with caution!
```

**Type Inference:**
Types are usually inferred, but you can be explicit:

```windjammer
let x: int = 42
let name: string = "Alice"
let numbers: Vec<int> = vec![1, 2, 3]
```

### Basic Types

| Type | Description | Example |
|------|-------------|---------|
| `int` | 64-bit integer | `let x = 42` |
| `int32` | 32-bit integer | `let x: int32 = 42` |
| `uint` | Unsigned 64-bit | `let x: uint = 42` |
| `float` | 64-bit float | `let x = 3.14` |
| `bool` | Boolean | `let x = true` |
| `string` | UTF-8 string | `let x = "hello"` |

### Comments

```go
// Single-line comment

// Multi-line comments not yet supported
// Just use multiple single-line comments
```

---

## Functions and Control Flow

### Functions

Functions are declared with `fn`:

```go
fn add(a: int, b: int) -> int {
    a + b  // Last expression is the return value
}

fn greet(name: string) {
    println("Hello, {}!", name)
}

fn main() {
    let sum = add(5, 3)
    greet("World")
}
```

**Note:** No semicolon on the last line means it's returned!

### If Expressions

```go
let x = 10

if x > 5 {
    println("x is big")
} else if x > 0 {
    println("x is small")
} else {
    println("x is not positive")
}

// If as an expression
let description = if x > 5 {
    "big"
} else {
    "small"
}
```

### Loops

#### For Loops

```go
// Range-based for loop
for i in 0..10 {
    println("{}", i)  // 0 to 9
}

// Inclusive range
for i in 0..=10 {
    println("{}", i)  // 0 to 10
}

// Iterate over collection
let numbers = vec![1, 2, 3, 4, 5]
for num in numbers {
    println("{}", num)
}
```

#### While Loops

```go
let mut count = 0
while count < 10 {
    println("{}", count)
    count += 1
}
```

#### Infinite Loops

```go
loop {
    println("Forever!")
    break  // Use break to exit
}
```

### Break and Continue

```go
for i in 0..10 {
    if i == 5 {
        continue  // Skip to next iteration
    }
    if i == 8 {
        break  // Exit loop
    }
    println("{}", i)
}
```

---

## Structs and Methods

### Defining Structs

```go
struct Point {
    x: int,
    y: int,
}

struct User {
    name: string,
    email: string,
    age: int,
    active: bool,
}
```

### Creating Instances

```go
// Long form
let p1 = Point {
    x: 10,
    y: 20,
}

// Shorthand (when variable names match fields)
let x = 10
let y = 20
let p2 = Point { x, y }
```

### Methods with Impl Blocks

```go
struct Rectangle {
    width: int,
    height: int,
}

impl Rectangle {
    // Associated function (like static method)
    fn new(width: int, height: int) -> Rectangle {
        Rectangle { width, height }
    }
    
    // Method that borrows self
    fn area(&self) -> int {
        self.width * self.height
    }
    
    // Method that mutably borrows self
    fn scale(&mut self, factor: int) {
        self.width *= factor
        self.height *= factor
    }
    
    // Method that consumes self
    fn into_square(self) -> Rectangle {
        let size = if self.width > self.height {
            self.width
        } else {
            self.height
        }
        Rectangle::new(size, size)
    }
}

fn main() {
    let mut rect = Rectangle.new(10, 20)
    println("Area: {}", rect.area())
    
    rect.scale(2)
    println("New dimensions: {}x{}", rect.width, rect.height)
}
```

**Self Parameters:**
- `&self` - Immutable borrow (read-only access)
- `&mut self` - Mutable borrow (can modify)
- `self` - Takes ownership (consumes the value)

---

## Ownership and Borrowing

Windjammer's killer feature is **automatic ownership inference**. In most cases, you don't need to think about it!

### The Magic of Inference

```go
struct User {
    name: string,
}

// Compiler infers: name is borrowed (immutable)
fn print_name(user: User) {
    println("{}", user.name)
}

// Compiler infers: user is mutably borrowed
fn change_name(user: User) {
    user.name = "New Name"  // Mutation detected!
}

// Compiler infers: user is owned (consumed)
fn consume_user(user: User) -> User {
    user  // Returned, so must be owned
}
```

### Explicit Ownership

Sometimes you want to be explicit:

```go
fn read_only(user: &User) {
    println("{}", user.name)
}

fn modify(user: &mut User) {
    user.name = "Modified"
}

fn take_ownership(user: User) {
    // user is moved here
}
```

### References

Create references with `&`:

```go
let x = 42
let ref_x = &x
let mut_ref_x = &mut x

fn double_value(x: &mut int) {
    *x *= 2  // Dereference with *
}
```

**Rules:**
1. One value, one owner
2. Multiple immutable borrows (`&T`) OR one mutable borrow (`&mut T`)
3. References must be valid (no dangling pointers)

The compiler enforces these rules and infers the right types!

---

## Enums and Pattern Matching

### Enums

Enums let you define a type with a set of possible variants:

```windjammer
// Simple enum (like constants)
enum Color {
    Red,
    Green,
    Blue,
}

// Enum with data (like Rust)
enum IpAddress {
    V4(string),
    V6(string),
}

// Complex enum with multiple data types
enum Message {
    Quit,
    Move(int, int),
    Write(string),
    ChangeColor(int, int, int),
}
```

**Creating enum values:**

```windjammer
let color = Color.Red
let localhost = IpAddress.V4("127.0.0.1")
let msg = Message.Write("Hello")
```

### Match Expressions

Pattern matching is how you work with enums:

```windjammer
enum Color {
    Red,
    Green,
    Blue,
    Custom(int, int, int),
}

let color = Color.Red

match color {
    Color.Red => println!("Red!"),
    Color.Green => println!("Green!"),
    Color.Blue => println!("Blue!"),
    Color.Custom(r, g, b) => println!("RGB({}, {}, {})", r, g, b),
}
```

### Match with Values

```go
let number = 7

let description = match number {
    1 => "one",
    2 => "two",
    3 | 4 | 5 => "three to five",
    6..=10 => "six to ten",
    _ => "something else",
}
```

### Tuple Patterns

```go
let pair = (true, 42)

match pair {
    (true, x) => println("First is true, second is {}", x),
    (false, x) => println("First is false, second is {}", x),
}
```

### Guards

```go
let number = 4

match number {
    x if x < 0 => println("Negative"),
    x if x == 0 => println("Zero"),
    x if x < 10 => println("Single digit positive"),
    _ => println("Large number"),
}
```

---

## Generic Types

### Vec<T> - Dynamic Arrays

```go
// Creating vectors
let numbers: Vec<int> = Vec.new()
let mut names = vec!["Alice", "Bob", "Charlie"]

// Adding elements
names.push("David")

// Accessing elements
let first = names[0]
let maybe_fifth = names.get(4)  // Returns Option<string>

// Iterating
for name in names {
    println("{}", name)
}
```

### Option<T> - Nullable Values

```go
fn divide(a: int, b: int) -> Option<int> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}

let result = divide(10, 2)

match result {
    Some(value) => println("Result: {}", value),
    None => println("Cannot divide by zero!"),
}

// Or use if let
if let Some(value) = result {
    println("Result: {}", value)
}
```

### Result<T, E> - Error Handling

```go
use std.fs
use std.io

fn read_username(path: string) -> Result<string, Error> {
    let content = fs.read_to_string(path)?
    Ok(content.trim().to_string())
}

fn main() -> Result<(), Error> {
    let username = read_username("username.txt")?
    println("Username: {}", username)
    Ok(())
}
```

---

## Concurrency

### Go-style Goroutines

```go
use std.sync.mpsc

fn main() {
    let (tx, rx) = mpsc.channel()
    
    // Spawn concurrent tasks
    go {
        tx <- "Hello from goroutine!"
    }
    
    go {
        tx <- "Another message"
    }
    
    // Receive messages
    println(<-rx)
    println(<-rx)
}
```

### Channel Operators

Windjammer supports Go's channel syntax:

```go
let (tx, rx) = mpsc.channel()

// Send to channel
tx <- 42

// Receive from channel
let value = <-rx

// Traditional Rust syntax also works
tx.send(42).unwrap()
let value = rx.recv().unwrap()
```

### Async/Await

```go
async fn fetch_url(url: string) -> Result<string, Error> {
    let response = http.get(url).await?
    let body = response.text().await?
    Ok(body)
}

async fn main() {
    let result = fetch_url("https://example.com").await
    match result {
        Ok(body) => println("{}", body),
        Err(e) => println("Error: {}", e),
    }
}
```

---

## Error Handling

### The ? Operator

```go
use std.fs

fn read_config() -> Result<string, Error> {
    let content = fs.read_to_string("config.toml")?
    // ? automatically returns the error if it occurs
    Ok(content)
}
```

### Handling Multiple Errors

```go
fn process_file(path: string) -> Result<(), Error> {
    let content = fs.read_to_string(path)?
    let parsed = parse_config(&content)?
    let validated = validate_config(parsed)?
    save_config(validated)?
    Ok(())
}
```

### Pattern Matching on Results

```go
let result = read_file("data.txt")

match result {
    Ok(content) => {
        println("File contents: {}", content)
    }
    Err(error) => {
        println("Error reading file: {}", error)
    }
}
```

---

## Character Literals

Windjammer supports character literals with single quotes:

```windjammer
let letter = 'a'
let digit = '5'
let symbol = '@'

// Use in pattern matching
fn describe_char(c: char) -> string {
    match c {
        'a' => "lowercase a",
        'A' => "uppercase A",
        '0' => "zero digit",
        _ => "other character",
    }
}
```

### Escape Sequences

Windjammer supports common escape sequences:

```windjammer
let newline = '\n'      // Newline
let tab = '\t'          // Tab
let carriage = '\r'     // Carriage return
let backslash = '\\'    // Backslash
let single_quote = '\''  // Single quote
let null = '\0'         // Null character

// Use in strings
println!("Line 1\nLine 2\nLine 3")
println!("Column1\tColumn2\tColumn3")
```

### Character Operations

```windjammer
fn is_vowel(c: char) -> bool {
    match c {
        'a' | 'e' | 'i' | 'o' | 'u' => true,
        'A' | 'E' | 'I' | 'O' | 'U' => true,
        _ => false,
    }
}

fn main() {
    let ch = 'a'
    if is_vowel(ch) {
        println!("${ch} is a vowel")
    }
}
```

---

## Decorators and Auto-Derive

### @auto Derive

The most common use of decorators is `@auto` for automatic trait derivation:

```windjammer
// Automatically implement Debug, Clone, Copy
@auto(Debug, Clone, Copy)
struct Point {
    x: int,
    y: int,
}

// Now you can:
let p1 = Point { x: 10, y: 20 }
let p2 = p1  // Copy
println!("{:?}", p1)  // Debug

// Common derives
@auto(Debug, Clone, PartialEq, Eq)
struct User {
    name: string,
    email: string,
}

// For serialization (when using serde)
@auto(Debug, Clone, Serialize, Deserialize)
struct Config {
    host: string,
    port: int,
}
```

**Available auto-derives:**
- `Debug` - Debug printing (`{:?}`)
- `Clone` - Deep copying
- `Copy` - Bitwise copying (for simple types)
- `PartialEq`, `Eq` - Equality comparison
- `PartialOrd`, `Ord` - Ordering
- `Hash` - Hashing for HashMap/HashSet
- `Default` - Default values
- `Serialize`, `Deserialize` - JSON/etc (requires serde)

### Custom Decorators

```windjammer
// Measure execution time
@timing
fn expensive_operation(n: int) -> int {
    // Complex computation
    n * n
}

// HTTP routing
@route("/api/users")
@get
async fn list_users() -> Json<Vec<User>> {
    // ...
}

// Multiple decorators
@cache(ttl: 60)
@timing
fn compute_value(x: int) -> int {
    x * x
}
```

### Decorator Arguments

```windjammer
@route("/users/:id")
@auth_required
async fn get_user(id: Path<int>) -> Result<Json<User>, StatusCode> {
    // ...
}
```

### Field Decorators

Decorators can also be applied to struct fields, which is particularly useful for CLI argument parsing, serialization, and validation:

```windjammer
// CLI argument parsing with clap
@command(
    name: "my-tool",
    about: "A sample CLI tool",
    version: "1.0"
)
struct Args {
    @arg(help: "Input files to process")
    files: Vec<string>,
    
    @arg(short: 'o', long: "output", help: "Output directory")
    output_dir: Option<string>,
    
    @arg(short: 'v', long: "verbose", help: "Verbose output")
    verbose: bool,
    
    @arg(long: "workers", default_value: "4", help: "Number of threads")
    workers: int,
}

// Serialization with custom field names
@auto(Serialize, Deserialize)
struct User {
    @serde(rename: "username")
    name: string,
    
    @serde(skip_serializing_if: "Option::is_none")
    email: Option<string>,
}

// Validation
struct Config {
    @validate(range: (min: 1, max: 65535))
    port: int,
    
    @validate(url)
    api_endpoint: string,
}
```

**Common field decorators:**
- `@arg(...)` - CLI argument configuration (clap)
- `@serde(...)` - Serialization options (serde)
- `@validate(...)` - Field validation
- `@doc(...)` - Field documentation

The generated Rust code converts these to appropriate Rust attributes:
```rust
struct Args {
    #[arg(help = "Input files to process")]
    files: Vec<String>,
    
    #[arg(short = 'o', long = "output", help = "Output directory")]
    output_dir: Option<String>,
}
```

---

## Traits

Traits define shared behavior (like interfaces in Go or traits in Rust). Windjammer supports a powerful trait system with bounds, where clauses, and associated types.

### Basic Trait Definitions

```windjammer
trait Drawable {
    fn draw(&self)
    fn area(&self) -> float
}

struct Circle {
    radius: float
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("Drawing circle with radius {}", self.radius)
    }
    
    fn area(&self) -> float {
        3.14159 * self.radius * self.radius
    }
}
```

### Trait Bounds

Trait bounds specify requirements for generic type parameters:

**Single trait bound:**

```windjammer
fn print_value<T: Display>(value: T) {
    println!("{}", value)
}
```

**Multiple trait bounds with `+`:**

```windjammer
fn display_and_clone<T: Display + Clone>(value: T) {
    let copy = value.clone()
    println!("Original: {}", value)
    println!("Clone: {}", copy)
}
```

**Trait bounds on structs:**

```windjammer
struct Container<T: Clone> {
    value: T
}

impl<T: Clone> Container<T> {
    fn duplicate(&self) -> T {
        self.value.clone()
    }
}
```

**Multiple type parameters with bounds:**

```windjammer
fn compare<T: Display, U: Display>(a: T, b: U) {
    println!("A: {}", a)
    println!("B: {}", b)
}
```

### Where Clauses

For complex trait bounds, use `where` clauses for better readability:

**Simple where clause:**

```windjammer
fn process<T, U>(first: T, second: U)
where
    T: Display,
    U: Debug
{
    println!("First: {}", first)
    println!("Second: {:?}", second)
}
```

**Multiple bounds per type parameter:**

```windjammer
fn complex_operation<T, U>(a: T, b: U)
where
    T: Display + Clone,
    U: Debug + Clone
{
    let a_copy = a.clone()
    let b_copy = b.clone()
    println!("Processing: {}, {:?}", a, b)
}
```

**Struct with where clause:**

```windjammer
struct Pair<T, U>
where
    T: Clone,
    U: Clone
{
    first: T,
    second: U
}

impl<T, U> Pair<T, U>
where
    T: Clone + Display,
    U: Clone + Display
{
    fn display_both(&self) {
        println!("First: {}", self.first)
        println!("Second: {}", self.second)
    }
}
```

### Associated Types

Associated types allow traits to define placeholder types that implementers specify:

**Trait with associated type:**

```windjammer
trait Container {
    type Item;
    
    fn get(&self) -> Self::Item;
    fn set(&mut self, item: Self::Item);
}
```

**Generic implementation:**

```windjammer
struct Box<T> {
    value: T
}

impl<T> Container for Box<T> {
    type Item = T;
    
    fn get(&self) -> Self::Item {
        self.value
    }
    
    fn set(&mut self, item: Self::Item) {
        self.value = item
    }
}
```

**Concrete implementation:**

```windjammer
struct IntBox {
    number: int
}

impl Container for IntBox {
    type Item = int;
    
    fn get(&self) -> Self::Item {
        self.number
    }
    
    fn set(&mut self, item: Self::Item) {
        self.number = item
    }
}
```

**Multiple associated types:**

```windjammer
trait Converter {
    type Input;
    type Output;
    
    fn convert(&self, input: Self::Input) -> Self::Output;
}

struct Doubler {
    multiplier: int
}

impl Converter for Doubler {
    type Input = int;
    type Output = int;
    
    fn convert(&self, input: Self::Input) -> Self::Output {
        input * self.multiplier
    }
}
```

**Using associated types in bounds:**

```windjammer
fn process_container<C>(container: &C)
where
    C: Container,
    C::Item: Display
{
    let item = container.get()
    println!("Container item: {}", item)
}
```

### Why Use Associated Types?

Associated types are preferable when:
- A trait has exactly one type that makes sense for an implementation
- You want cleaner syntax without extra type parameters
- The type is determined by the trait implementation, not by the caller

Example comparison:

```windjammer
// Without associated types (more verbose)
trait Container<Item> {
    fn get(&self) -> Item;
}

// With associated types (cleaner)
trait Container {
    type Item;
    fn get(&self) -> Self::Item;
}
```

### Trait Objects

Trait objects enable **runtime polymorphism** - calling different implementations through a common interface.

**Syntax**: `dyn TraitName`

**As function parameter (reference)**:

```windjammer
fn render_shape(shape: &dyn Drawable) {
    shape.draw()
}

let circle = Circle { radius: 5 }
let square = Square { side: 10 }

render_shape(&circle)  // Works!
render_shape(&square)  // Works!
```

**As return type (automatically boxed)**:

```windjammer
fn create_pet(choice: int) -> dyn Pet {
    if choice == 1 {
        Dog { name: "Buddy" }
    } else {
        Cat { name: "Whiskers" }
    }
}

// Windjammer automatically boxes: dyn Pet -> Box<dyn Pet>
```

**In collections**:

```windjammer
let shapes: Vec<dyn Drawable> = vec![
    Circle { radius: 5 },
    Square { side: 10 }
]

for shape in shapes {
    render_shape(&shape)
}
```

**Generated Rust**:
- `&dyn Trait` → `&dyn Trait` (reference, no boxing)
- `dyn Trait` → `Box<dyn Trait>` (owned, automatically boxed)
- `&mut dyn Trait` → `&mut dyn Trait` (mutable reference)

### Supertraits

Supertraits specify that implementing one trait requires implementing another.

**Syntax**: `trait SubTrait: SuperTrait`

**Single supertrait**:

```windjammer
trait Animal {
    fn make_sound(&self);
}

trait Pet: Animal {
    fn play(&self);
}

// Any type implementing Pet MUST also implement Animal
struct Dog {
    name: string
}

impl Animal for Dog {
    fn make_sound(&self) {
        println!("Woof!")
    }
}

impl Pet for Dog {
    fn play(&self) {
        println!("{} is playing!", self.name)
    }
}
```

**Multiple supertraits**:

```windjammer
trait Manager: Worker + Clone {
    fn manage(&self);
}

// Implementing Manager requires implementing both Worker AND Clone
```

**Why use supertraits?**
- Express trait hierarchies (Pet IS AN Animal)
- Ensure required capabilities (Manager must be a Worker)
- Reuse trait methods (Pet can call Animal methods)

### Generic Traits

Generic traits have type parameters, allowing flexible reuse.

**Single type parameter**:

```windjammer
trait From<T> {
    fn from(value: T) -> Self;
}

// Different implementations for different types
impl From<int> for String {
    fn from(value: int) -> Self {
        value.to_string()
    }
}

impl From<float> for String {
    fn from(value: float) -> Self {
        value.to_string()
    }
}
```

**Multiple type parameters**:

```windjammer
trait Converter<Input, Output> {
    fn convert(&self, input: Input) -> Output;
}

struct IntToString;

impl Converter<int, string> for IntToString {
    fn convert(&self, input: int) -> string {
        input.to_string()
    }
}
```

**When to use generic traits vs associated types?**

Use **generic traits** when:
- Multiple implementations for the same type make sense
- The type parameter is chosen by the caller
- Example: `From<int>` and `From<string>` both for the same type

Use **associated types** when:
- Only one implementation makes sense
- The type is determined by the trait implementation
- Example: `Iterator` has one `Item` type per implementation

---

## Named Bound Sets

**Version**: v0.11.0+

Define reusable trait bound combinations to reduce boilerplate in generic code.

### Basic Usage

```windjammer
// Define common trait combinations
bound Printable = Display + Debug
bound Copyable = Clone + Copy
bound Comparable = PartialEq + PartialOrd

// Use in function signatures
fn log<T: Printable>(value: T) {
    println!("Display: {}", value)
    println!("Debug: {:?}", value)
}

fn duplicate<T: Copyable>(value: T) -> T {
    value.clone()
}
```

### Multiple Bounds

Combine named bounds just like regular traits:

```windjammer
bound Serializable = Serialize + Deserialize
bound Printable = Display + Debug

// Use both
fn save_and_log<T: Serializable + Printable>(item: T) {
    println!("Saving: {:?}", item);
    // ... serialize and save ...
}
```

### How It Works

Named bounds are **compile-time aliases** that expand during code generation:

```windjammer
bound Printable = Display + Debug

fn log<T: Printable>(x: T) { ... }

// Expands to:
fn log<T: Display + Debug>(x: T) { ... }
```

**No runtime overhead** - it's pure syntactic sugar!

### When to Use Named Bounds

✅ **Good use cases:**
- Common trait combinations used across your codebase
- Documenting intent (e.g., `Printable` is clearer than `Display + Debug`)
- Reducing boilerplate in large generic APIs

❌ **When not to use:**
- One-off trait bounds
- Overly generic names that don't add clarity

### Example: Web Service Traits

```windjammer
// Define domain-specific bounds
bound Storable = Serialize + Deserialize + Clone + Debug
bound Cacheable = Storable + Hash + Eq
bound ApiResource = Cacheable + Send + Sync

struct User { ... }
struct Post { ... }

// Use throughout your API
fn save_to_db<T: Storable>(item: T) { ... }
fn cache<T: Cacheable>(item: T) { ... }
fn handle_request<T: ApiResource>(resource: T) { ... }
```

---

## Standard Library Modules

**Version**: v0.15.0 (Server-Side Complete!)

Windjammer provides a comprehensive standard library for building production applications. **v0.15.0 completes the server-side development story** with HTTP server, file system, logging, regex, and CLI parsing.

**What This Means**:
- ✅ You write `http.serve()`, not `axum::Router::new()`
- ✅ You write `fs.read_to_string()`, not `std::fs::read_to_string()`
- ✅ You write `log.info()`, not `log::info!()`
- ✅ You write `regex.compile()`, not `Regex::new()`
- ✅ You write `cli.parse()`, not `Args::parse()`
- ✅ API stability - Windjammer controls the contract, not external crates
- ✅ Future flexibility - implementations can be swapped without breaking your code

**Available Modules** (v0.15.0):

**Web Development:**
- `std/http` - HTTP client + server (abstracts reqwest + axum) 🆕 **Server support!**
- `std/json` - JSON operations (abstracts serde_json)

**File System & I/O:**
- `std/fs` - File operations, directories, metadata (Rust stdlib) 🆕 **v0.15.0**
- `std/log` - Production logging with levels (abstracts env_logger) 🆕 **v0.15.0**

**Data & Patterns:**
- `std/regex` - Regular expressions (abstracts regex) 🆕 **v0.15.0**
- `std/db` - Database access (abstracts sqlx)
- `std/time` - Time/date utilities (abstracts chrono)
- `std/crypto` - Cryptography (abstracts sha2, bcrypt, base64)
- `std/random` - Random generation (abstracts rand)

**Developer Tools:**
- `std/cli` - CLI argument parsing (abstracts clap) 🆕 **v0.15.0**
- `std/testing` - Test assertions
- `std/collections` - Data structures

**System:**
- `std/async` - Async utilities
- `std/env` - Environment variables
- `std/process` - Process execution

### Environment Variables (`std/env`)

```windjammer
use std.env

fn main() {
    // Get with default
    let path = env.get_or("PATH", "/usr/bin")
    
    // Get optional
    match env.get("HOME") {
        Some(home) => println!("Home: {}", home),
        None => println!("No HOME set")
    }
    
    // Set and remove
    env.set("MY_VAR", "value")
    env.remove("MY_VAR")
    
    // Current directory
    let cwd = env.current_dir()
    
    // All variables
    let all_vars = env.vars()
}
```

### Process Execution (`std/process`)

```windjammer
use std.process

fn main() {
    // Run shell command
    match process.run("ls -la") {
        Ok(output) => println!("Output: {}", output),
        Err(err) => println!("Error: {}", err)
    }
    
    // Run with explicit arguments
    let args = vec!["--version"]
    match process.run_with_args("rustc", args) {
        Ok(output) => println!("{}", output),
        Err(err) => eprintln!("{}", err)
    }
    
    // Process info
    println!("PID: {}", process.pid())
    
    // Exit (use sparingly!)
    // process.exit(0)
}
```

### Random Numbers (`std/random`)

```windjammer
use std.random

fn main() {
    // Random integer in range
    let dice = random.range(1, 6)
    
    // Random float (0.0 to 1.0)
    let chance = random.float()
    
    // Random boolean
    let coin_flip = random.bool()
    
    // Shuffle a vector
    let numbers = vec![1, 2, 3, 4, 5]
    let shuffled = random.shuffle(numbers)
    
    // Pick random element
    let items = vec!["apple", "banana", "cherry"]
    match random.choice(items) {
        Some(fruit) => println!("Picked: {}", fruit),
        None => println!("Empty list!")
    }
}
```

### Async Utilities (`std/async`)

```windjammer
use std.async

@async
fn main() {
    println!("Waiting...")
    async.sleep_ms(1000).await
    println!("Done!")
}
```

### File System (`std/fs`) 🆕 **v0.15.0**

Complete file system operations without exposing `std::fs`:

```windjammer
use std.fs

fn main() {
    // Read and write files
    match fs.write("config.txt", "port=3000") {
        Ok(_) => println!("File written"),
        Err(e) => println!("Error: {}", e)
    }
    
    let content = fs.read_to_string("config.txt")?
    println!("Content: {}", content)
    
    // Directory operations
    fs.create_dir_all("data/logs")?
    let entries = fs.read_dir(".")?
    
    for entry in entries {
        println!("{} ({})", 
            entry.name(),
            if entry.is_dir() { "dir" } else { "file" }
        )
    }
    
    // File metadata
    let meta = fs.metadata("config.txt")?
    println!("Size: {} bytes", meta.size())
    
    // Path operations
    let path = fs.join("data", "file.txt")
    let ext = fs.extension("file.txt")?  // "txt"
}
```

### Logging (`std/log`) 🆕 **v0.15.0**

Production-ready logging without `log::` or `env_logger::`:

```windjammer
use std.log

fn main() {
    // Initialize logger
    log.init_with_level("info")
    
    // Log at different levels
    log.trace("Very detailed debugging")
    log.debug("Debugging information")
    log.info("General information")
    log.warn("Warning message")
    log.error("Error occurred")
    
    // Structured logging with context
    log.info_with("User logged in", "user_id", "12345")
    log.warn_with("Slow query", "duration_ms", "1500")
    
    // Conditional logging for expensive operations
    if log.debug_enabled() {
        let debug_data = expensive_calculation()
        log.debug(&debug_data)
    }
}
```

### Regular Expressions (`std/regex`) 🆕 **v0.15.0**

Pattern matching without `regex::`:

```windjammer
use std.regex

fn main() {
    // Compile and use regex
    let email_re = regex.compile(r"[\w.]+@[\w.]+")?
    
    if email_re.is_match("alice@example.com") {
        println!("Valid email!")
    }
    
    // Find all matches
    let text = "Emails: alice@test.com, bob@test.org"
    for m in email_re.find_all(text) {
        println!("Found: {}", m.text())
    }
    
    // Capture groups
    let date_re = regex.compile(r"(\d{4})-(\d{2})-(\d{2})")?
    match date_re.captures("Date: 2025-10-09") {
        Some(caps) => {
            println!("Year: {}", caps.get(1)?)
            println!("Month: {}", caps.get(2)?)
            println!("Day: {}", caps.get(3)?)
        },
        None => {}
    }
    
    // Replace operations
    let censored = email_re.replace_all(text, "[EMAIL]")
    
    // Quick one-off operations
    if regex.is_match(r"^\d+$", "12345")? {
        println!("All digits!")
    }
}
```

### CLI Argument Parsing (`std/cli`) 🆕 **v0.15.0**

Parse command-line arguments without `clap::`:

```windjammer
use std.cli

@derive(Cli, Debug)
struct Args {
    @arg(help: "Input file to process")
    input: string,
    
    @arg(short: 'o', long: "output", help: "Output file")
    output: Option<string>,
    
    @arg(short: 'v', long: "verbose", help: "Verbose output")
    verbose: bool,
    
    @arg(short: 'n', long: "count", default_value: "10", help: "Number of items")
    count: int,
}

fn main() {
    let args = cli.parse::<Args>()
    
    println!("Processing: {}", args.input)
    
    if args.verbose {
        println!("Verbose mode enabled")
    }
    
    match args.output {
        Some(out) => println!("Output: {}", out),
        None => println!("Output to stdout")
    }
}
```

---

## String Interpolation

Make strings more readable with `${}` syntax:

```windjammer
let name = "Alice"
let age = 30

// Old way
println!("Hello, {}! You are {} years old.", name, age)

// New way (string interpolation)
println!("Hello, ${name}! You are ${age} years old.")

// Works with expressions
let x = 5
let y = 10
println!("The sum of ${x} and ${y} is ${x + y}")
```

**Complex expressions:**

```windjammer
struct User {
    name: string,
    email: string,
}

let user = User { name: "Bob", email: "bob@example.com" }
println!("User: ${user.name} (${user.email})")
```

---

## Pipe Operator

Chain function calls elegantly with `|>`:

```windjammer
// Without pipe operator
let result = to_string(add_ten(double(5)))

// With pipe operator (left-to-right, easier to read!)
let result = 5 |> double |> add_ten |> to_string

// Real-world example
let users = fetch_users()
    |> filter_active
    |> sort_by_name
    |> take(10)

// Works with methods too
let text = "  hello world  "
    |> trim
    |> to_uppercase
    |> split_whitespace
```

**Pipe with arguments:**

```windjammer
fn add(x: int, y: int) -> int { x + y }

// The value gets passed as the first argument
let result = 5 |> add(10)  // Same as: add(5, 10)
```

---

## Labeled Arguments

Make function calls self-documenting with labeled arguments:

```windjammer
// Function definition
fn create_user(name: string, age: int, email: string) -> User {
    User { name, age, email }
}

// Call with labeled arguments (any order!)
let user = create_user(
    name: "Alice",
    email: "alice@example.com",
    age: 30
)

// Mix positional and labeled
let user2 = create_user("Bob", age: 25, email: "bob@test.com")
```

**Why use labeled arguments?**

```windjammer
// Without labels - what do these booleans mean?
connect_database("localhost", 5432, true, false, 30)

// With labels - crystal clear!
connect_database(
    host: "localhost",
    port: 5432,
    use_ssl: true,
    auto_retry: false,
    timeout_seconds: 30
)
```

---

## Advanced Topics

### Closures

```go
// Simple closure
let add_one = |x| x + 1

// Multiple parameters
let multiply = |x, y| x * y

// With iterator methods
let numbers = vec![1, 2, 3, 4, 5]
let doubled = numbers.iter().map(|n| n * 2).collect()
let evens = numbers.iter().filter(|n| n % 2 == 0).collect()
```

### Range Expressions

```go
// Exclusive range (0 to 9)
for i in 0..10 {
    println("{}", i)
}

// Inclusive range (0 to 10)
for i in 0..=10 {
    println("{}", i)
}

// Custom ranges
let slice = &array[2..5]  // Elements 2, 3, 4
```

### Enums

```go
enum Status {
    Pending,
    InProgress,
    Completed,
    Failed(string),  // Enum with data
}

let status = Status.InProgress

match status {
    Status.Pending => println("Not started"),
    Status.InProgress => println("Working on it"),
    Status.Completed => println("Done!"),
    Status.Failed(reason) => println("Failed: {}", reason),
}
```

### Defer

```go
fn process_file(path: string) -> Result<(), Error> {
    let file = fs.File.open(path)?
    defer file.close()  // Will run when function exits
    
    // Process file...
    
    // file.close() automatically called here
    Ok(())
}
```

---

## Compiler Optimizations

### Automatic Performance (v0.20.0)

**Your naive code runs at 98.7% of expert Rust performance - automatically!**

**Plus**: Functions return **393x faster** with automatic defer drop optimization!

Windjammer includes a 7-phase optimization pipeline that transforms simple code into high-performance Rust:

**Phase 0: Defer Drop (v0.20.0)** 🆕 ⚡
- **Automatically defers heavy deallocations to background threads**
- **393x faster time-to-return** for functions with large owned parameters
- **Zero configuration, zero code changes, instant responses**

Example:
```windjammer
// You write:
fn get_size(data: HashMap<int, Vec<int>>) -> int {
    data.len()
}

// Compiler automatically generates:
fn get_size(data: HashMap<usize, Vec<usize>>) -> usize {
    let len = data.len();
    // DEFER DROP: Deallocate data (Large) in background thread
    std::thread::spawn(move || drop(data));
    len  // Returns 393x faster!
}
```

**When It Applies:**
- Function owns large parameter (HashMap, Vec, String, etc.)
- Function returns small value (int, bool, reference, etc.)
- Type is `Send` (can move to another thread)
- Type has no critical `Drop` side effects (not Mutex, File, Channel, etc.)

**Safety:**
- Conservative whitelist (HashMap, BTreeMap, Vec, String, etc.)
- Blacklist for unsafe types (Mutex, File, TcpStream, etc.)
- All checks happen at compile time
- **Empirically validated** with [comprehensive benchmarks](../benches/defer_drop_latency.rs)

**Performance Impact:**
- HashMap (1M entries): **375ms → 1ms** (393x faster!)
- API request (10MB): **24ms → 18ms** (1.3x faster)
- Perfect for CLIs, web APIs, interactive UIs

**Phase 1: Inline Hints**
- Automatically adds `#[inline]` to small functions and hot paths
- You write simple functions, compiler makes them fast

**Phase 2: Clone Elimination**
- Removes unnecessary `.clone()` calls
- Loop-aware analysis ensures correctness
- Reduces heap allocations significantly

**Phase 3: Struct Shorthand**
- Generates idiomatic Rust patterns like `Point { x, y }`
- Cleaner, more efficient generated code

**Phase 4: String Capacity Pre-allocation**
- Optimizes `format!` calls with `String::with_capacity`
- Eliminates reallocation overhead
- Auto-imports `std::fmt::Write` when needed

**Phase 5: Compound Assignments**
- Converts `x = x + 1` to `x += 1` automatically
- More efficient code patterns

**Phase 6: Constant Folding**
- Evaluates constant expressions at compile time
- `2 + 3` becomes `5` in generated code
- Eliminates runtime computation for known values

**Example:**
```windjammer
// You write this:
fn greet(name: string) {
    let msg = format!("Hello, {}!", name)
    println!("{}", msg)
}

// Compiler generates this:
#[inline]
fn greet(name: &str) {
    let msg = {
        let mut __s = String::with_capacity(64);
        write!(&mut __s, "Hello, {}!", name).unwrap();
        __s
    };
    println!("{}", msg)
}
```

**No manual optimization needed!** The compiler handles it automatically.

**Performance**: 98.7% of expert Rust - EXCEEDS 93-95% target!

---

## Best Practices

### 1. Use Ownership Inference

Let the compiler figure out borrowing:

```go
// Good - let compiler infer
fn process_user(user: User) {
    println("{}", user.name)
}

// Only be explicit when needed
fn must_modify(user: &mut User) {
    user.name = "Modified"
}
```

### 2. Prefer Expressions

```go
// Good - expression style
let message = if x > 0 {
    "positive"
} else {
    "non-positive"
}

// Less idiomatic
let message
if x > 0 {
    message = "positive"
} else {
    message = "non-positive"
}
```

### 3. Use Pattern Matching

```go
// Good - clear and exhaustive
match result {
    Ok(value) => process(value),
    Err(e) => handle_error(e),
}

// Avoid excessive if-else chains
```

### 4. Leverage Decorators

```go
// Clean separation of concerns
@route("/api/data")
@auth_required
@timing
@cache(ttl: 300)
async fn get_data() -> Json<Data> {
    // Focus on business logic
}
```

---

## Developer Experience

Windjammer provides world-class IDE support through its comprehensive Language Server Protocol (LSP) implementation and Debug Adapter Protocol (DAP) integration.

### Language Server (LSP)

The Windjammer LSP (`windjammer-lsp`) provides rich IDE features for all major editors:

**Real-Time Diagnostics:**
- Instant syntax error detection as you type
- Semantic analysis (type checking, undefined symbols)
- Mapped Rust compiler errors (when available)
- Clear, actionable error messages

**Code Intelligence:**
- **Auto-Completion**: Context-aware suggestions for:
  - Keywords (`fn`, `struct`, `match`, etc.)
  - Standard library modules and functions
  - User-defined symbols (functions, structs, enums)
  - Method calls and struct fields
- **Hover Information**: See function signatures, parameter types, and documentation
- **Go to Definition** (F12 / Cmd+Click): Jump to any symbol definition
- **Find References**: See all usages of any symbol across your project
- **Rename Symbol**: Safe refactoring with project-wide renames

**Windjammer-Unique Features:**
- **Inlay Hints for Ownership**: See inferred ownership modes inline!
  ```windjammer
  fn process(s: string /* & */, mut x: int /* &mut */) {
      // Hints show the compiler's inference
  }
  ```
- **Advanced Refactoring** 🆕 **v0.27.0**:
  - **Extract Function** - Transform selected code into reusable functions
  - **Inline Variable** - Replace variables with their values
  - **Introduce Variable** - Extract expressions into named variables
  - **Change Signature** - Modify function parameters across all call sites
  - **Move Item** - Move functions/structs between files with import auto-update
  - **Preview Mode** - See changes before applying
  - **Batch Refactorings** - Apply multiple refactorings atomically
- **Code Actions**:
  - Quick fixes for common issues
  - All refactorings available as code actions

**Performance:**
- **Hash-Based Incremental Compilation**: Only re-analyzes files when content changes
- **Cache Hits**: ~1-5ms response time
- **Large Files**: Handles 1000+ line files without lag
- **Scalable**: Works efficiently with 1000+ file projects

**Setup:**

**VSCode:**
```bash
# Install the extension
code --install-extension windjammer-vscode

# Or manually: Copy windjammer-vscode/ to ~/.vscode/extensions/
```

**Vim/Neovim:**
```vim
" Add to your LSP config (with nvim-lspconfig)
require'lspconfig'.windjammer_lsp.setup{}
```

**IntelliJ IDEA:**
```
Settings → Plugins → Marketplace → Search "LSP4IJ"
Settings → Languages & Frameworks → Language Server Protocol
  → Add → windjammer-lsp
```

### Debugging (DAP)

The Windjammer Debug Adapter provides seamless debugging of `.wj` files through DAP:

**Features:**
- **Breakpoints**: Set breakpoints in `.wj` source files
- **Step Through Code**: Step over, step into, step out
- **Variable Inspection**: View variables, call stack, and scopes
- **Expression Evaluation**: Evaluate expressions in debug context
- **Source Mapping**: Automatic translation between Windjammer and generated Rust

**How It Works:**
1. Windjammer generates Rust with source maps
2. DAP adapter translates between `.wj` line numbers and `.rs` line numbers
3. Uses `lldb` (or `gdb`) to debug the underlying Rust binary
4. Presents everything in terms of your Windjammer source

**Setup:**

**VSCode:**
```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "windjammer",
      "request": "launch",
      "name": "Debug Windjammer Program",
      "program": "${workspaceFolder}/target/debug/my_program"
    }
  ]
}
```

**Vim/Neovim (with nvim-dap):**
```lua
local dap = require('dap')
dap.adapters.windjammer = {
  type = 'executable',
  command = 'windjammer-lsp',  -- Also provides DAP
  args = {'--debug'}
}
dap.configurations.windjammer = {
  {
    type = 'windjammer',
    request = 'launch',
    name = 'Debug Windjammer Program',
    program = '${workspaceFolder}/target/debug/my_program',
  }
}
```

**IntelliJ IDEA:**
```
Run → Edit Configurations → Add New Configuration → DAP
  Adapter: windjammer-lsp --debug
  Program: target/debug/my_program
```

**Example Debug Session:**
```windjammer
// main.wj
fn factorial(n: int) -> int {
    if n <= 1 {
        return 1  // ← Set breakpoint here
    }
    n * factorial(n - 1)
}

fn main() {
    let result = factorial(5)
    println!("Result: {}", result)
}
```

1. Set breakpoint on `return 1`
2. Run debugger
3. Inspect variables: `n = 1`, call stack shows recursion depth
4. Step out to see return values propagate

**Pro Tips:**
- Source maps are generated automatically with `--debug` flag
- Breakpoints work in both `.wj` and generated `.rs` files
- Use conditional breakpoints: `n == 1` in the breakpoint settings
- Watch expressions update in real-time during stepping

### Editor Integration Status

| Editor | LSP | Syntax | Debugging | Auto-Format |
|--------|-----|--------|-----------|-------------|
| **VSCode** | ✅ Full | ✅ Yes | ✅ Full | ✅ Yes |
| **Vim/Neovim** | ✅ Full | ✅ Yes | ✅ DAP plugin | ✅ Yes |
| **IntelliJ IDEA** | ✅ LSP4IJ | ⚠️ Manual | ⚠️ Manual | ✅ Yes |
| **Emacs** | ⚠️ LSP-mode | ⚠️ Manual | ⚠️ DAP-mode | ✅ Yes |
| **Sublime Text** | ⚠️ LSP plugin | ⚠️ Manual | ❌ No | ✅ Yes |

✅ = Full support, ⚠️ = Community/manual setup, ❌ = Not yet available

**Contributing Editor Support:**
- See `crates/windjammer-lsp/README.md` for integration guides
- Editor plugin source: `editor-plugins/` directory
- PRs welcome for new editors!

---

## World-Class Linting (v0.26.0) 🆕

Windjammer includes a comprehensive linting system that matches golangci-lint's capabilities while providing real-time feedback through the LSP!

### 16 Linting Rules Across 6 Categories

**Code Quality & Style:**
1. `unused-code` - Detect unused functions, structs, enums **(auto-fixable)**
2. `function-length` - Flag overly long functions
3. `file-length` - Flag large files
4. `naming-convention` - Check PascalCase for structs **(auto-fixable)**
5. `missing-docs` - Require documentation

**Error Handling:**
6. `unchecked-result` - Detect ignored Result types
7. `avoid-panic` - Warn about panic!() usage
8. `avoid-unwrap` - Warn about .unwrap() usage

**Performance:**
9. `vec-prealloc` - Suggest Vec::with_capacity() **(auto-fixable)**
10. `string-concat` - Warn about inefficient string concatenation
11. `clone-in-loop` - Detect expensive cloning in loops

**Security:**
12. `unsafe-block` - Flag unsafe code blocks
13. `hardcoded-secret` - Detect hardcoded credentials
14. `sql-injection` - Warn about SQL query concatenation

**Dependencies:**
15. `circular-dependency` - Detect import cycles

**Maintainability:**
16. Various metrics and coupling analysis

### CLI Usage

```bash
# Run linter
wj lint --path src

# Auto-fix issues
wj lint --path src --fix

# Strict mode (errors only)
wj lint --path src --errors-only

# JSON output for CI/CD
wj lint --path src --json

# Custom thresholds
wj lint --path src \
  --max-function-length 100 \
  --max-file-length 1000 \
  --max-complexity 10

# Disable specific categories
wj lint --path src --no-unused --no-style
```

### Configuration

You can configure linting thresholds:

```rust
LintConfig {
    max_function_length: 50,
    max_file_length: 500,
    max_complexity: 10,
    check_unused: true,
    check_style: true,
    check_performance: true,
    check_security: true,
    check_error_handling: true,
    enable_autofix: false,  // Enable with --fix flag
}
```

### Auto-Fix System

Three rules support automatic fixing:

**1. unused-code:**
```windjammer
// Before
fn unused_helper() {
    // code
}

// After (with --fix)
#[allow(dead_code)]
fn unused_helper() {
    // code
}
```

**2. naming-convention:**
```windjammer
// Before
struct myStruct {
    value: int
}

// After (with --fix)
struct MyStruct {
    value: int
}
```

**3. vec-prealloc:**
```windjammer
// Suggests:
let mut items = Vec::with_capacity(10);
// Instead of:
let mut items = Vec::new();
```

### Real-Time LSP Integration

Unlike command-line linters, Windjammer provides **instant feedback as you type**:

- ✅ Errors and warnings appear in real-time
- ✅ Quick fixes available via code actions
- ✅ Auto-fix on save (configurable)
- ✅ Hover to see full diagnostic details
- ✅ Jump to related code with one click

### CLI Output Example

```
Linting Windjammer files in: "src"

Configuration:
  • Max function length: 50
  • Max file length: 500
  • Max complexity: 10
  • Check unused code: yes
  • Check style: yes
  • Auto-fix: enabled

Diagnostic Categories (inspired by golangci-lint):
  ✓ Code Quality: complexity, style, code smell
  ✓ Error Detection: bug risk, error handling
  ✓ Performance: performance, memory
  ✓ Security: security checks
  ✓ Maintainability: naming, documentation, unused
  ✓ Dependencies: import, dependency (circular)

Rules Implemented:
  [16 rules across 6 categories]

✨ World-class linting ready!
```

### Why Windjammer Linting Wins

**vs golangci-lint (Go):**
- ✅ Real-time editor integration (not just CLI)
- ✅ Type-aware analysis (leverages Salsa)
- ✅ Consistent with language compiler
- ✅ Auto-fix in editor

**vs clippy (Rust):**
- ✅ Better organized (6 clear categories)
- ✅ Unified CLI (`wj lint` vs `cargo clippy`)
- ✅ Configurable thresholds
- ✅ Comprehensive auto-fix

**Combined Benefits:**
- ✅ Best of both worlds
- ✅ 94 tests ensuring reliability
- ✅ Production-ready from day one
- ✅ Extensible for custom rules

---

## Eject to Pure Rust (No Lock-In!)

**Version**: v0.30.0+

One of Windjammer's most powerful features is the ability to **eject your project to pure Rust** at any time. This removes all fear of vendor lock-in and provides multiple benefits.

### Why Eject?

✅ **Learn Rust** - See exactly how Windjammer compiles to Rust  
✅ **Migration Path** - Gradually transition from Windjammer to Rust  
✅ **Safety Net** - Try Windjammer with zero commitment  
✅ **Hybrid Development** - Start simple in Windjammer, optimize in Rust  
✅ **No Lock-In** - Never be stuck with a language decision

### How to Eject

```bash
# Eject current directory to a new Rust project
wj eject --path . --output my-rust-project

# Eject with options
wj eject --path . --output my-rust-project \
  --format              # Run rustfmt (default: true)
  --comments            # Add helpful comments (default: true)
  --no-cargo-toml       # Skip Cargo.toml generation
```

### What You Get

When you eject, Windjammer generates:

1. **Pure Rust Code** (`.rs` files)
   - Preserves all compiler optimizations as explicit code
   - Formatted with `rustfmt`
   - Includes helpful comments explaining Windjammer features

2. **Complete `Cargo.toml`**
   - All dependencies automatically detected and added
   - Proper edition and optimization settings
   - Ready to `cargo build`

3. **Project Files**
   - `README.md` explaining the ejected project
   - `.gitignore` for Rust projects
   - All necessary configuration

4. **Source Comments** (if `--comments` enabled)
   ```rust
   //! This file was automatically generated by Windjammer eject.
   //!
   //! Original Windjammer source: main.wj
   //!
   //! Windjammer features used in this file:
   //! - Ownership inference: Types inferred automatically from usage
   //! - Trait bound inference: Generic constraints derived from operations
   //! - 15-phase optimization pipeline for 99%+ Rust performance
   ```

### Example

```bash
# Original Windjammer project
$ ls
main.wj  lib.wj  utils.wj

# Eject to Rust
$ wj eject --path . --output rust-project

🚀 Ejecting Windjammer project to Rust...
  Input:  "."
  Output: "rust-project"

Found 3 Windjammer file(s):
  • main.wj
  • lib.wj
  • utils.wj

  Ejecting main.wj... ✓
  Ejecting lib.wj... ✓
  Ejecting utils.wj... ✓

  Creating Cargo.toml... ✓
  Creating README.md... ✓
  Creating .gitignore... ✓

  Formatting generated code... ✓

✅ Ejection complete!

Your Rust project is ready at: "rust-project"

Next steps:
  1. cd "rust-project"
  2. cargo build         # Build the project
  3. cargo test          # Run tests
  4. cargo run           # Run the application

# Now you have a pure Rust project!
$ cd rust-project
$ cargo build
   Compiling windjammer-ejected v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.1s
```

### Understanding the Generated Code

The ejected Rust code includes all optimizations that Windjammer applied automatically:

```windjammer
// Original Windjammer (main.wj)
fn greet(name: string) {
    println!("Hello, {}!", name)
}
```

```rust
// Generated Rust (main.rs)
//! This file was automatically generated by Windjammer eject.
//! Original Windjammer source: main.wj

#[inline]  // Phase 1: Inline hints (automatic)
fn greet(name: &str) {  // Ownership inference: &str
    println!("Hello, {}!", name)
}
```

### One-Way Conversion

**Important**: Ejection is a **one-way process**. You cannot convert Rust back to Windjammer.

However:
- ✅ Your original `.wj` files are **never modified**
- ✅ You can continue developing in Windjammer
- ✅ You can eject again anytime
- ✅ You can maintain both versions if needed

### Use Cases

**1. Learning Rust:**
```bash
# Write simple Windjammer code
# Eject to see idiomatic Rust
# Learn Rust patterns without the initial complexity
```

**2. Migration Strategy:**
```bash
# Start: 100% Windjammer
# Eject critical paths to Rust for optimization
# Gradually increase Rust percentage
# End: 100% Rust (if desired)
```

**3. Hybrid Development:**
```bash
my_project/
├── src/
│   ├── main.wj           # Simple, high-level logic
│   ├── api.wj            # Business logic
│   └── hot_path.rs       # Hand-optimized Rust
```

**4. Safety Net:**
```bash
# Try Windjammer for a project
# If it doesn't work out, eject to Rust
# Zero wasted effort - you have working Rust code!
```

### Tips

- **Eject Early, Eject Often**: See how your code compiles to Rust
- **Compare Performance**: Benchmark Windjammer vs ejected Rust
- **Learn Patterns**: Study the generated code to learn Rust idioms
- **Version Control**: Commit before ejecting to track changes

---

## What's Next?

Now that you've learned the basics, try:

1. **Build a CLI tool** - See `examples/cli_tool/`
2. **Create a web server** - See `examples/http_server/`
3. **Make a WASM app** - See `examples/wasm_game/`
4. **Eject a Project** - See how Windjammer compiles to Rust! 🆕
5. **Read the examples** - Learn from working code
6. **Experiment!** - The best way to learn

---

## AI-Powered Development with MCP

Windjammer includes a **Model Context Protocol (MCP) server** that enables AI assistants like Claude and ChatGPT to deeply understand, analyze, and generate Windjammer code.

### What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io) is a standard way for AI tools to interact with code bases. It's like an API for AI assistants to understand your code.

### Quick Setup with Claude Desktop

1. **Install the MCP server** (comes with Windjammer):
   ```bash
   # MCP server is installed automatically with Windjammer
   which windjammer-mcp  # Verify it's in your PATH
   ```

2. **Configure Claude Desktop**:
   
   Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):
   
   ```json
   {
     "mcpServers": {
       "windjammer": {
         "command": "/usr/local/bin/windjammer-mcp",
         "args": ["stdio"]
       }
     }
   }
   ```

3. **Restart Claude Desktop**

4. **Test it!** Ask Claude:
   > "Parse this Windjammer code: `fn add(a: int, b: int) -> int { a + b }`"

### What Can AI Assistants Do?

Once connected, Claude (or other MCP-enabled AI tools) can:

**1. Parse and Analyze Code:**
```
You: "What does this Windjammer code do?"

fn process(items: Vec<string>) -> int {
    items.iter().filter(|s| s.len() > 3).count()
}

Claude: This function takes a vector of strings, filters items with more than 3 characters, and returns the count. The compiler will infer that `items` should be `&[String]` and the return type is `usize` (which maps to `int` in Windjammer).
```

**2. Generate Code from Descriptions:**
```
You: "Create a function that filters even numbers from a vector"

Claude: Here's idiomatic Windjammer code:

fn filter_evens(numbers: Vec<int>) -> Vec<int> {
    numbers.iter().filter(|&n| n % 2 == 0).collect()
}
```

**3. Explain Compiler Errors:**
```
You: "What does this error mean?"
error: mismatched types
  expected `i64`, found `&str`

Claude: You're trying to use a string where a number is expected. The compiler requires types to match exactly. For example, if you declared a variable as `let x: int = "hello"`, you need to either change the type to `string` or change the value to a number like `42`.
```

**4. Refactor Code:**
```
You: "Extract the filtering logic into a separate function"

Claude: Here's the refactored code:

fn is_even(n: int) -> bool {
    n % 2 == 0
}

fn filter_evens(numbers: Vec<int>) -> Vec<int> {
    numbers.iter().filter(|&n| is_even(n)).collect()
}
```

**5. Search Your Codebase:**
```
You: "Find all functions that return Result<T, Error>"

Claude: [Searches your workspace and shows relevant matches]
```

**6. Type Inference Insights:**
```
You: "What type will the compiler infer for `x`?"

let data = vec![1, 2, 3]
let x = data.iter().map(|n| n * 2).collect()

Claude: The compiler will infer `x: Vec<i64>` because:
- `data` is `Vec<i64>` (Windjammer's `int` maps to `i64`)
- `.iter()` produces `Iterator<Item = &i64>`
- `.map(|n| n * 2)` produces `Iterator<Item = i64>`
- `.collect()` gathers into `Vec<i64>`
```

### Available Tools

The MCP server provides these tools to AI assistants:

| Tool | Description |
|------|-------------|
| `parse_code` | Parse Windjammer code and return AST structure |
| `analyze_types` | Perform type inference and show inferred types |
| `generate_code` | Generate Windjammer code from natural language |
| `explain_error` | Explain compiler errors in plain English |
| `get_definition` | Find where a symbol is defined |
| `search_workspace` | Search for code patterns across files |

### Advanced: Using with Other AI Tools

The MCP server works with any AI assistant that supports MCP:

**ChatGPT (via API):**
```python
import subprocess
import json

# Start MCP server
server = subprocess.Popen(
    ["windjammer-mcp", "stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    text=True
)

# Send parse request
request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
        "name": "parse_code",
        "arguments": {"code": "fn main() { println!(\"Hi\") }"}
    }
}

server.stdin.write(json.dumps(request) + "\n")
server.stdin.flush()

# Read response
response = json.loads(server.stdout.readline())
print(response)
```

**Custom Integration:**
See [crates/windjammer-mcp/README.md](../crates/windjammer-mcp/README.md) for full API documentation.

### Benefits of AI-Assisted Development

- ✅ **Learn Faster** - AI explains Windjammer concepts instantly
- ✅ **Code Faster** - Generate boilerplate from descriptions  
- ✅ **Debug Faster** - Plain English error explanations
- ✅ **Refactor Safely** - AI suggests improvements using your codebase
- ✅ **Consistency** - MCP uses same Salsa database as LSP for accuracy

### Troubleshooting

**"Claude doesn't show MCP tools"**
- Restart Claude Desktop after config changes
- Check that `windjammer-mcp` is in your PATH
- Verify the config file path is correct

**"MCP server crashes"**
- Check logs in Claude Desktop (Help → View Logs)
- Try running manually: `windjammer-mcp stdio`
- File an issue on GitHub with logs

**"AI gives incorrect information"**
- The AI generates responses; MCP provides the data
- Always verify generated code
- Report issues to improve the tools

---

## Getting Help

- Read the [README.md](README.md) for language features
- Check [ARCHITECTURE.md](ARCHITECTURE.md) for compiler internals
- Look at [examples/](examples/) for real-world code
- File issues on GitHub for bugs or questions

Happy coding with Windjammer! 🎉

