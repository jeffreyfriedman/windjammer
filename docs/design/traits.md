# Windjammer Trait System Design

## Philosophy

Windjammer uses **Rust-style traits** with **ergonomic enhancements** to reduce verbosity while maintaining clarity and type safety.

**Core Principle**: The compiler should infer as much as possible, but allow explicit annotations when needed.

---

## Basic Syntax

### Trait Definition
```go
trait Drawable {
    fn draw(&self)
    fn area(&self) -> f64
}
```

### Trait Implementation
```go
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
```

### Generic Functions with Traits
```go
fn render<T: Drawable>(shape: &T) {
    shape.draw()
    println!("Area: ${shape.area()}")
}
```

---

## Ergonomic Features

### 1. Automatic Trait Bound Inference ⭐️

**The compiler analyzes function bodies and automatically infers required trait bounds.**

```go
// You write:
fn process<T>(item: T) {
    println!("{:?}", item)        // Uses Debug
    let copy = item.clone()       // Uses Clone
    copy
}

// Compiler generates:
fn process<T: Clone + Debug>(item: T) -> T {
    println!("{:?}", item);
    let copy = item.clone();
    copy
}
```

**Inference Rules:**
- `.clone()` → `Clone`
- `println!("{:?}", x)` → `Debug`
- `println!("{}", x)` → `Display`
- `x == y` → `PartialEq`
- `x < y`, `x > y` → `PartialOrd`
- `x.iter()` → `IntoIterator`
- Arithmetic ops → `Add`, `Sub`, `Mul`, `Div`

**Explicit Override:**
```go
// When inference isn't enough or you want to be explicit:
fn process<T: Clone + Debug + Send>(item: T) {
    // ...
}
```

---

### 2. Associated Types as Generic Parameters ⭐️

**Simpler syntax for associated types.**

```go
// Windjammer - clean syntax
trait Iterator<Item> {
    fn next(&mut self) -> Option<Item>
}

impl Iterator<int> for Counter {
    fn next(&mut self) -> Option<int> {
        // ...
    }
}

// Transpiles to Rust:
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

impl Iterator for Counter {
    type Item = i64;
    fn next(&mut self) -> Option<Self::Item> { ... }
}
```

**Detection Rule:**
- If a generic parameter appears in trait definition but NOT in `impl TypeName for Trait`, it's an associated type
- Convert to Rust's `type Name = ...` syntax during codegen

---

### 3. Auto-Derive Based on Usage ⭐️

**Automatically derive common traits based on how the type is used.**

```go
@auto
struct Point {
    x: int,
    y: int,
}

// Later in code:
let p1 = Point { x: 1, y: 2 }
let p2 = p1.clone()      // Auto-adds #[derive(Clone)]
if p1 == p2 { }          // Auto-adds #[derive(PartialEq)]
println!("{:?}", p1)     // Auto-adds #[derive(Debug)]
```

**Explicit Alternative:**
```go
@derive(Clone, Debug, PartialEq, Eq)
struct Point {
    x: int,
    y: int,
}
```

---

### 4. Trait Aliases (Composition)

**Bundle commonly used trait combinations.**

```go
// Define reusable trait bundles
trait Printable = Display + Debug
trait Comparable = PartialEq + PartialOrd
trait Numeric = Add + Sub + Mul + Div + PartialOrd

// Use them
fn show<T: Printable>(item: T) {
    println!("{}", item)
    println!("{:?}", item)
}

// Transpiles to:
fn show<T: Display + Debug>(item: T) { ... }
```

---

### 5. Ownership Inference for Trait Methods

**The compiler infers `self` vs `&self` vs `&mut self` based on method body.**

```go
trait Container<T> {
    fn get(self, index: int) -> T
    fn set(self, index: int, value: T)
    fn len(self) -> int
}

impl Container<int> for Vec<int> {
    fn get(self, index: int) -> int {
        self[index]              // Read-only → compiler infers &self
    }
    
    fn set(self, index: int, value: int) {
        self[index] = value      // Mutates → compiler infers &mut self
    }
    
    fn len(self) -> int {
        self.len()               // Read-only → compiler infers &self
    }
}

// Generated Rust has correct &self / &mut self
```

---

### 6. Simplified Trait Objects

**Automatic boxing for trait objects.**

```go
// Instead of: Box<dyn Display>
let item: Display = get_displayable()

// Instead of: Vec<Box<dyn Drawable>>
let shapes: Vec<Drawable> = vec![
    Circle { radius: 5.0 },
    Rectangle { width: 10, height: 20 },
]

// Compiler automatically adds Box<dyn ...> when needed
```

---

### 7. Where Clause Improvements

**Cleaner formatting, but primarily rely on inference.**

```go
// Explicit when needed
fn complex<T, U>(a: T, b: U)
    where T: Clone + Debug,
          U: Display
{
    // ...
}

// But prefer inference:
fn complex<T, U>(a: T, b: U) {
    let x = a.clone()      // Infers T: Clone
    println!("{:?}", a)    // Infers T: Debug
    println!("{}", b)      // Infers U: Display
}
```

---

## Complete Example

### Windjammer Code:
```go
@auto
struct Point {
    x: f64,
    y: f64,
}

trait Shape {
    fn area(self) -> f64
    fn perimeter(self) -> f64
}

impl Shape for Point {
    fn area(self) -> f64 {
        0.0  // Point has no area
    }
    
    fn perimeter(self) -> f64 {
        0.0
    }
}

// Generic function with inferred bounds
fn describe<T>(shape: T) {
    println!("Area: ${shape.area()}")           // Infers T: Shape
    println!("Perimeter: ${shape.perimeter()}")
    let copy = shape.clone()                    // Infers T: Clone
    println!("Copy: {:?}", copy)                // Infers T: Debug
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    describe(p)
    
    // Usage triggers @auto derivation:
    // - Clone (from describe)
    // - Debug (from describe)
}
```

### Generated Rust:
```rust
#[derive(Clone, Debug)]
struct Point {
    x: f64,
    y: f64,
}

trait Shape {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
}

impl Shape for Point {
    fn area(&self) -> f64 {
        0.0
    }
    
    fn perimeter(&self) -> f64 {
        0.0
    }
}

fn describe<T: Shape + Clone + Debug>(shape: T) {
    println!("Area: {}", shape.area());
    println!("Perimeter: {}", shape.perimeter());
    let copy = shape.clone();
    println!("Copy: {:?}", copy);
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    describe(p);
}
```

---

## Implementation Phases

### Phase 1: Core Trait System
- [ ] Parse trait definitions
- [ ] Parse `impl Trait for Type` blocks
- [ ] Basic trait bounds in generics
- [ ] Codegen for traits

### Phase 2: Ergonomic Features (Priority)
- [ ] **Trait bound inference from method calls**
- [ ] **Associated types as generic parameters**
- [ ] **@auto derive based on usage**
- [ ] Ownership inference for trait methods

### Phase 3: Advanced Features
- [ ] Trait aliases
- [ ] Automatic trait object boxing
- [ ] Default trait implementations
- [ ] Multiple trait bounds with `+`

### Phase 4: Full Rust Feature Parity
- [ ] Where clauses
- [ ] Marker traits
- [ ] Supertraits
- [ ] Associated constants
- [ ] Generic associated types (GATs)

---

## Design Decisions

### Why Explicit Implementation?

We chose explicit `impl Trait for Type` over Go-style implicit satisfaction because:

1. **Transparency**: Developers should understand the Rust being generated
2. **Clarity**: No ambiguity about what implements what
3. **Orphan Rule**: Rust's orphan rule prevents implementing external traits for external types
4. **Type Safety**: Explicit is safer and more maintainable

### Why Inference?

We add inference for trait bounds because:

1. **DRY**: Don't repeat yourself - the compiler can see what traits are needed
2. **Less Boilerplate**: 80% of trait bounds are obvious from usage
3. **Maintainability**: When adding new trait methods, bounds update automatically
4. **Beginner Friendly**: New users don't need to know all the trait names

### When to Be Explicit

Use explicit trait bounds when:
- You want to document API requirements clearly
- Inference can't determine all needed bounds
- You need bounds for Send/Sync in async contexts
- You're exposing a public API

---

## Benefits

**For Beginners:**
- Less intimidating than raw Rust traits
- Inference handles most cases automatically
- Clear error messages about missing traits

**For Experienced Developers:**
- Can be as explicit as needed
- Full Rust trait system power available
- Cleaner, more maintainable code

**For Everyone:**
- Less boilerplate
- Better readability
- Same runtime performance as Rust

---

## Future Considerations

### Trait Specialization
When Rust stabilizes trait specialization, we can support:
```go
impl<T> Display for Option<T> {
    // Default implementation
}

impl Display for Option<String> {
    // Specialized for String
}
```

### Async Traits
Built-in support for async trait methods:
```go
trait AsyncReader {
    async fn read(self, buf: &mut [u8]) -> Result<usize, Error>
}
```

Transpiles using `async-trait` crate or native Rust async traits when stable.

---

## Comparison: Rust vs Windjammer

| Feature | Rust | Windjammer |
|---------|------|------------|
| Trait definition | `trait Name { }` | `trait Name { }` |
| Implementation | `impl Trait for Type` | `impl Trait for Type` |
| Trait bounds | `<T: Trait>` explicit | `<T>` inferred or explicit |
| Associated types | `type Item;` | `<Item>` in trait definition |
| Auto-derive | `#[derive(...)]` | `@derive(...)` or `@auto` |
| Trait objects | `Box<dyn Trait>` | `Trait` (auto-boxed) |
| Method ownership | Must specify | Inferred or explicit |
| Where clauses | Required for complex | Optional, prefer inference |

---

## Summary

Windjammer traits provide **Rust's power with better ergonomics**:

✅ Explicit implementation (no magic)  
✅ Automatic trait bound inference (less boilerplate)  
✅ Cleaner associated type syntax  
✅ Smart auto-derive  
✅ Ownership inference  
✅ Direct mapping to Rust traits  

**Result**: Write less, express more, maintain type safety.

