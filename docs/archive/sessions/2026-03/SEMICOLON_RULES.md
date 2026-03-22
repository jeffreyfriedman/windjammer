# Windjammer Syntax Rules: Semicolons and `mut`

## TL;DR

**Semicolons are OPTIONAL everywhere in Windjammer.**  
**The `mut` keyword is OPTIONAL for local variables - mutability is inferred automatically.**

This aligns with modern languages (Swift, Kotlin, Go) and the Windjammer philosophy of maximizing ergonomics.

## The Rules

### ✅ Optional for Statements
```windjammer
let x = 5           // No semicolon needed
let y = 10          // No semicolon needed
x = x + y           // No semicolon needed
return x            // No semicolon needed
```

### ✅ Optional for Associated Types (NEW in v0.38.6)
```windjammer
trait Add {
    type Output     // No semicolon needed (like Swift, Kotlin, Go)
    fn add(self, other: Self) -> Self
}

impl Add for Vec2 {
    type Output = Vec2      // No semicolon needed
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}
```

### ✅ Optional for Function Declarations
```windjammer
fn add(a: int, b: int) -> int {
    a + b       // No semicolon needed (implicit return)
}
```

## Design Rationale

### Why Optional Semicolons?

1. **Modern Ergonomics**: Swift, Kotlin, Go, Python all have optional semicolons
2. **Less Noise**: Cleaner, more readable code
3. **Windjammer Philosophy**: "Hide complexity, maximize ergonomics"
4. **Already Proven**: The parser handles it correctly with no ambiguity

### Why Not Require Them Everywhere?

We considered requiring semicolons everywhere for consistency, but decided against it because:

1. **Goes against modern language design trends**
2. **Adds unnecessary boilerplate**
3. **Doesn't align with Windjammer's goal of reducing syntax noise**
4. **The parser already handles optional semicolons correctly**

### Comparison with Other Languages

#### Swift (Apple's modern language)
```swift
let x = 5        // No semicolon
let y = 10       // No semicolon

protocol Add {
    associatedtype Output  // No semicolon!
    func add(_ other: Self) -> Output
}
```

#### Kotlin (JetBrains' modern language)
```kotlin
val x = 5        // No semicolon
val y = 10       // No semicolon

interface Add {
    fun add(other: Add): Add  // No semicolon
}
```

#### Go (Google's modern language)
```go
x := 5        // No semicolon (automatic insertion)
y := 10       // No semicolon

type Adder interface {
    Add(other Adder) Adder  // No semicolon
}
```

#### Rust (systems language - the outlier)
```rust
let x = 5;        // Semicolon required for statements
let y = 10;       // Semicolon required

trait Add {
    type Output;  // Semicolon REQUIRED (Rust quirk)
    fn add(self, other: Self) -> Self::Output;
}
```

**Windjammer chose to follow Swift/Kotlin/Go, not Rust's quirk.**

## Historical Context

### v0.38.5 and Earlier
Associated types **required** semicolons (copying Rust):
```windjammer
impl Add for Vec2 {
    type Output = Vec2;  // Semicolon required
    // ...
}
```

This was inconsistent with:
- Statements (semicolons optional)
- Modern language design (Swift, Kotlin, Go)
- Windjammer's ergonomics philosophy

### v0.38.6 and Later
Associated types now have **optional** semicolons:
```windjammer
impl Add for Vec2 {
    type Output = Vec2  // Semicolon optional
    // ...
}
```

This is now consistent with:
- All other Windjammer syntax
- Modern languages (Swift, Kotlin, Go)
- Windjammer's ergonomics philosophy

## Parser Implementation

The parser uses **lookahead** to determine statement boundaries without requiring semicolons:

```rust
// In parse_impl (item_parser.rs)
if self.current_token() == &Token::Type {
    self.advance(); // consume 'type'
    let assoc_name = /* parse name */;
    self.expect(Token::Assign)?;
    let concrete_type = self.parse_type()?;
    
    // Semicolons are optional
    if self.current_token() == &Token::Semicolon {
        self.advance(); // consume optional semicolon
    }
    
    // Continue parsing...
}
```

The parser knows the statement is complete when it sees:
- A new keyword (`fn`, `type`, `pub`, etc.)
- A closing brace (`}`)
- End of file

## When to Use Semicolons

### You CAN use them if you want:
```windjammer
let x = 5;          // Optional semicolon
let y = 10;         // Optional semicolon
type Output = Vec2; // Optional semicolon
```

### You DON'T NEED them:
```windjammer
let x = 5           // Clean!
let y = 10          // Clean!
type Output = Vec2  // Clean!
```

### Multiple statements on one line (edge case):
```windjammer
let x = 5; let y = 10; let z = 15  // Semicolons help here
```

But this is rare and generally discouraged for readability.

## Error Messages

If you forget a semicolon where the parser gets confused, you'll get a helpful error:

```
Error: Parse error: Expected 'fn' or '}', got 'let'
  --> vec2.wj:82:5
   |
82 |     let x = 5
83 |     let y = 10
   |     ^^^ unexpected 'let' here
   |
   = note: Did you forget a semicolon or newline?
```

(Note: Error message improvements are ongoing)

## Automatic `mut` Inference (NEW in v0.38.6)

### The Problem with Rust

Rust requires explicit `mut` for mutable variables:

```rust
let mut x = 5;  // Must write mut
x = 10;         // ✅ Works

let y = 5;      // Immutable
y = 10;         // ❌ Error
```

**This is annoying because:**
1. The compiler can easily infer mutability from usage
2. You often don't know if a variable will be mutated when you first declare it
3. It's extra boilerplate that modern languages don't require

### Windjammer's Solution: Automatic Inference

```windjammer
let x = 5       // No mut needed!
x = 10          // ✅ Works - compiler adds mut automatically

let y = 5       // Never reassigned
// y = 10 would make it mutable

let z = 0
for i in 0..10 {
    z = z + i   // ✅ Works - compiler adds mut automatically
}
```

**Generated Rust code:**
```rust
let mut x = 5;  // Automatically added mut!
x = 10;

let y = 5;      // No mut (never reassigned)

let mut z = 0;  // Automatically added mut!
for i in 0..10 {
    z += i;
}
```

### Comparison with Other Languages

#### Go (Windjammer's inspiration)
```go
x := 5      // Mutable by default
x = 10      // ✅ Works
```

#### Swift
```swift
var x = 5   // Mutable (var keyword)
x = 10      // ✅ Works

let y = 5   // Immutable (let keyword)
y = 10      // ❌ Error
```

#### Kotlin
```kotlin
var x = 5   // Mutable (var keyword)
x = 10      // ✅ Works

val y = 5   // Immutable (val keyword)
y = 10      // ❌ Error
```

#### Rust (the outlier)
```rust
let mut x = 5;  // Must write mut explicitly
x = 10;         // ✅ Works

let y = 5;      // Immutable
y = 10;         // ❌ Error
```

**Windjammer chose Go's approach: mutable by default, inferred from usage.**

### When to Use Explicit `mut`

You CAN still write `mut` explicitly if you want:

```windjammer
let mut x = 5   // Explicit mut (backwards compatible)
x = 10
```

But you don't need to! The compiler will add it automatically if the variable is reassigned.

### Benefits

1. **Less boilerplate** - No need to predict if a variable will be mutated
2. **Cleaner code** - Fewer keywords = easier to read
3. **Modern design** - Aligns with Go, Swift, Kotlin
4. **Backwards compatible** - Explicit `mut` still works

## Summary

**Windjammer's syntax rules are simple:**
- **Semicolons: Optional everywhere**
- **`mut` keyword: Optional for local variables (inferred automatically)**
- **Use them if you want, skip them if you don't**
- **Follows modern language design (Swift, Kotlin, Go)**
- **Aligns with Windjammer philosophy: maximize ergonomics, minimize syntax noise**

These decisions were made after careful consideration during dogfooding, when we realized we were copying Rust's quirks (required semicolons for associated types, explicit `mut` for local variables) without good reasons. Modern languages don't require them, and neither should Windjammer.

