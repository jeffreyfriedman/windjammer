# Windjammer Async Execution Model Design

**Status:** APPROVED - Implementation Pending  
**Version:** 2.0 (Breaking Change)  
**Date:** November 3, 2025  
**Author:** Design Discussion Session

---

## Executive Summary

Windjammer will adopt a **call-site explicit async model** that combines the best of Go's caller-controlled execution and Rust's type safety. Functions are defined synchronously by default, and callers decide execution mode using explicit markers (`async`, `spawn`) or context blocks.

**Key Principle:** The caller controls execution, not the function definition.

---

## Background

### Current Design (v1.0 - To Be Deprecated)

```windjammer
// Function declares it's async
async fn fetch_data(url: string) -> Result<Data> {
    http.get(url)
}

// Caller must handle async
let data = fetch_data(url).await
```

**Problems:**
1. Function dictates execution model
2. Creates "function coloring" (sync vs async)
3. Forces library authors to maintain two versions
4. Not aligned with Windjammer's Go-inspired philosophy

### Design Goals

1. **Caller Control:** Caller chooses sync, async, or parallel execution
2. **Type Safety:** Maintain Rust-level type guarantees
3. **Local Reasoning:** Code understandable without global context
4. **Performance Visibility:** Explicit where concurrency happens
5. **Zero Crate Leakage:** No tokio/async-std in user code
6. **Ergonomic:** Minimal boilerplate for common cases
7. **Testable:** Easy to write unit tests

---

## Design Alternatives Considered

### Alternative 1: Rust-Style Explicit `async fn`

```rust
async fn fetch() -> Data { }
```

**Pros:** Type safe, debuggable, predictable  
**Cons:** Function coloring, library duplication, not caller-controlled  
**Decision:** ❌ Rejected - Violates Windjammer philosophy

### Alternative 2: Context-Aware (Pure Inference)

```windjammer
fn fetch() { }

async { fetch() }  // Compiler infers this should be async
```

**Pros:** Ergonomic, caller-controlled  
**Cons:** Type system nightmare, hidden behavior, "spooky action at a distance"  
**Decision:** ❌ Rejected - Too many soundness issues

### Alternative 3: Call-Site Explicit (SELECTED)

```windjammer
fn fetch() { }

fetch()        // Sync
async fetch()  // Async
spawn fetch()  // Parallel
```

**Pros:** Type safe, caller-controlled, debuggable, ergonomic  
**Cons:** Requires new syntax  
**Decision:** ✅ **APPROVED** - Best balance of all goals

---

## Detailed Design

### 1. Function Definitions

**No `async` keyword on function definitions:**

```windjammer
// All functions are defined synchronously
fn fetch_user(id: int) -> User {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

fn process_data(data: Data) -> Result<Output> {
    // Just regular code
    transform(data)
}
```

**Why:**
- One function definition serves all execution modes
- Library authors write once, users choose execution
- No function coloring

### 2. Call-Site Execution Control

#### Synchronous (Default)

```windjammer
let user = fetch_user(1)  // Blocks until complete
```

- **Returns:** `User`
- **Behavior:** Blocking execution
- **Use Case:** Simple scripts, tests, local operations

#### Asynchronous (I/O Bound)

```windjammer
let future = async fetch_user(1)  // Returns immediately
let user = future.await            // Wait for completion
```

- **Returns:** `Future<User>`
- **Behavior:** Non-blocking, cooperative multitasking
- **Use Case:** Network I/O, database queries, HTTP handlers

#### Parallel (CPU Bound)

```windjammer
let handle = spawn fetch_user(1)  // Spawns OS thread
let user = handle.join()           // Wait for completion
```

- **Returns:** `JoinHandle<User>`
- **Behavior:** True parallelism on separate thread
- **Use Case:** CPU-intensive work, blocking I/O

### 3. Execution Blocks

For multiple operations, use blocks instead of repeating prefixes:

#### Async Block

```windjammer
async {
    let u1 = fetch_user(1)   // Implicitly: async fetch_user(1).await
    let u2 = fetch_user(2)   // Implicitly: async fetch_user(2).await
    process(u1, u2)          // Implicitly: async process(u1, u2).await
}
```

**Semantics:**
- Creates async context
- All function calls implicitly awaited
- Returns when all operations complete
- Runs on async runtime (tokio)

#### Thread Block

```windjammer
thread {
    let u1 = fetch_user(1)   // Runs in new thread
    let u2 = fetch_user(2)   // Sequential within thread
    process(u1, u2)          // Still in thread
}
```

**Semantics:**
- Spawns OS thread
- Code runs synchronously within thread
- Returns `JoinHandle` for the block
- Can be joined to get result

### 4. Decorator Context

Web framework decorators implicitly create async context:

```windjammer
@get("/users/:id")
fn get_user(id: int) -> Json<User> {
    // Body is implicitly wrapped in async { }
    let user = fetch_user(id)  // Auto-awaited
    Json(user)
}

// Equivalent to:
@get("/users/:id")  
fn get_user(id: int) -> Json<User> {
    async {
        let user = fetch_user(id)
        Json(user)
    }
}
```

**Decorators with async context:**
- `@get`, `@post`, `@put`, `@delete`, `@patch` - HTTP handlers
- `@websocket` - WebSocket handlers
- User-defined async decorators

**Rationale:** Web handlers are inherently async in Rust ecosystem. Making this explicit at decorator level is clearer than requiring `async` everywhere inside.

---

## Type System

### Return Type Transformations

| Call Syntax | Base Return | Actual Return |
|-------------|-------------|---------------|
| `foo()` | `T` | `T` |
| `async foo()` | `T` | `Future<T>` |
| `spawn foo()` | `T` | `JoinHandle<T>` |

### Type Checking Rules

```windjammer
fn fetch() -> User { }

// Type checking:
let u: User = fetch()                    // ✅ OK
let f: Future<User> = async fetch()      // ✅ OK
let h: JoinHandle<User> = spawn fetch()  // ✅ OK

let u: User = async fetch()              // ❌ Error: expected User, found Future<User>
let f: Future<User> = fetch()            // ❌ Error: expected Future<User>, found User
```

### In Blocks

```windjammer
async {
    let u: User = fetch()  // ✅ Type is User (auto-awaited)
}

thread {
    let u: User = fetch()  // ✅ Type is User (sync in thread)
}
```

---

## Implementation Strategy

### Phase 1: Parser Changes

**Remove:**
```rust
// OLD: async keyword on functions
Token::Async => parse_async_function()
```

**Add:**
```rust
// NEW: async/spawn as call-site prefixes
enum CallPrefix {
    None,
    Async,
    Spawn,
}

struct Call {
    prefix: CallPrefix,
    function: Expression,
    arguments: Vec<Expression>,
}
```

**Grammar:**
```ebnf
call_expr := call_prefix? primary_expr '(' args ')'
call_prefix := 'async' | 'spawn'

async_block := 'async' '{' statements '}'
thread_block := 'thread' '{' statements '}'
```

### Phase 2: AST Changes

```rust
// Expression variants
enum Expression {
    Call {
        prefix: CallPrefix,  // NEW
        function: Box<Expression>,
        arguments: Vec<(String, Expression)>,
    },
    AsyncBlock {             // EXISTS
        body: Vec<Statement>,
    },
    ThreadBlock {            // EXISTS
        body: Vec<Statement>,
    },
}

enum CallPrefix {
    None,
    Async,
    Spawn,
}
```

### Phase 3: Type Checking

```rust
impl TypeChecker {
    fn check_call(&mut self, call: &Call) -> Type {
        let base_type = self.check_expr(&call.function);
        
        match call.prefix {
            CallPrefix::None => base_type,
            CallPrefix::Async => Type::Future(Box::new(base_type)),
            CallPrefix::Spawn => Type::JoinHandle(Box::new(base_type)),
        }
    }
    
    fn check_async_block(&mut self, block: &AsyncBlock) -> Type {
        // Inside async block, track context
        self.in_async_context = true;
        let result = self.check_block(&block.body);
        self.in_async_context = false;
        
        // Async block returns Future<T>
        Type::Future(Box::new(result))
    }
}
```

### Phase 4: Code Generation

```rust
impl CodeGenerator {
    fn generate_call(&mut self, call: &Call) -> String {
        let func = self.generate_expr(&call.function);
        let args = self.generate_args(&call.arguments);
        
        match (&call.prefix, self.in_async_block) {
            // Outside async block
            (CallPrefix::None, false) => {
                format!("{}({})", func, args)
            }
            (CallPrefix::Async, false) => {
                format!("{}({}).await", func, args)
            }
            (CallPrefix::Spawn, false) => {
                format!("std::thread::spawn(move || {}({}))", func, args)
            }
            
            // Inside async block - implicit await
            (CallPrefix::None, true) => {
                format!("{}({}).await", func, args)
            }
            (CallPrefix::Async, true) => {
                // Explicit async in async block - spawn separate task
                format!("tokio::spawn(async move {{ {}({}).await }})", func, args)
            }
            (CallPrefix::Spawn, true) => {
                // Spawn thread from async context
                format!("std::thread::spawn(move || {}({}))", func, args)
            }
        }
    }
    
    fn generate_async_block(&mut self, block: &AsyncBlock) -> String {
        self.in_async_block = true;
        let body = self.generate_statements(&block.body);
        self.in_async_block = false;
        
        format!("tokio::spawn(async move {{\n{}\n}}).await.unwrap()", body)
    }
}
```

### Phase 5: Decorator Handling

```rust
impl CodeGenerator {
    fn generate_function(&mut self, func: &Function) -> String {
        // Check if any decorator creates async context
        let has_async_decorator = func.decorators.iter().any(|d| {
            matches!(d.name.as_str(), "get" | "post" | "put" | "delete" | "patch" | "websocket")
        });
        
        if has_async_decorator {
            // Generate as async fn, body gets implicit async context
            self.in_async_block = true;
            let body = self.generate_statements(&func.body);
            self.in_async_block = false;
            
            format!("async fn {}(...) {{\n{}\n}}", func.name, body)
        } else {
            // Regular sync function
            format!("fn {}(...) {{\n{}\n}}", func.name, body)
        }
    }
}
```

---

## Migration Guide

### Breaking Changes

**Old (v1.0):**
```windjammer
async fn fetch() -> Data { }

let data = fetch().await
```

**New (v2.0):**
```windjammer
fn fetch() -> Data { }

let data = async fetch().await
// OR
async {
    let data = fetch()
}
```

### Migration Steps

1. **Remove `async` from function definitions**
   ```diff
   - async fn fetch() -> Data { }
   + fn fetch() -> Data { }
   ```

2. **Add `async` at call sites**
   ```diff
   - let data = fetch().await
   + let data = async fetch().await
   ```

3. **Or use async blocks**
   ```diff
   - let d1 = fetch1().await
   - let d2 = fetch2().await
   + async {
   +     let d1 = fetch1()
   +     let d2 = fetch2()
   + }
   ```

4. **HTTP handlers - no change needed**
   ```windjammer
   @get("/")
   fn handler() -> Response {
       // Still works - decorator handles async context
   }
   ```

### Migration Tool

```bash
wj migrate async-v2 src/
```

Automatically:
- Removes `async` from function definitions
- Adds `async` prefix to call sites with `.await`
- Converts multiple awaits to async blocks
- Preserves decorator behavior

---

## Examples

### Example 1: Simple Async I/O

```windjammer
fn fetch_data(url: string) -> Result<Data> {
    http.get(url)
}

fn main() {
    // Sync
    let data = fetch_data("https://api.com").unwrap()
    println!("Got: {}", data)
    
    // Async
    async {
        let data = fetch_data("https://api.com").unwrap()
        println!("Got: {}", data)
    }
}
```

### Example 2: Parallel Processing

```windjammer
fn process_item(item: Item) -> Result {
    // CPU-intensive work
    heavy_computation(item)
}

fn main() {
    let items = load_items()
    let handles = vec![]
    
    for item in items {
        let handle = spawn process_item(item)
        handles.push(handle)
    }
    
    for handle in handles {
        let result = handle.join().unwrap()
        println!("Result: {}", result)
    }
}
```

### Example 3: Web Handler

```windjammer
fn fetch_user(id: int) -> Result<User> {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

@get("/users/:id")
fn get_user(id: int) -> Json<User> {
    // Implicitly async context due to @get
    let user = fetch_user(id).unwrap()
    Json(user)
}
```

### Example 4: Mixed Execution

```windjammer
fn main() {
    // Fetch 10 users concurrently (I/O bound)
    let users = async {
        let mut results = vec![]
        for id in 1..10 {
            let user = fetch_user(id).unwrap()
            results.push(user)
        }
        results
    }
    
    // Process them in parallel (CPU bound)
    let handles = vec![]
    for user in users {
        let handle = spawn process_user(user)
        handles.push(handle)
    }
    
    // Collect results
    for handle in handles {
        let result = handle.join().unwrap()
        save_result(result)  // Sync write
    }
}
```

### Example 5: Library Function

```windjammer
// Library provides ONE function
pub fn read_config(path: string) -> Result<Config> {
    fs.read_to_string(path).map(|s| parse_config(s))
}

// Users choose execution mode:
let cfg = read_config("app.toml")        // Sync: blocks until done
let cfg = async read_config("app.toml")  // Async: non-blocking
let cfg = spawn read_config("app.toml")  // Parallel: background thread
```

---

## Performance Implications

### Memory Overhead

| Execution Mode | Overhead | Notes |
|----------------|----------|-------|
| Sync | 0 bytes | Just function call |
| Async | ~100 bytes | Future state machine |
| Spawn | ~2 MB | OS thread stack |

### When to Use What

**Use Sync (default):**
- Local computation
- Small operations
- Testing
- Scripts

**Use Async:**
- Network I/O
- Database queries
- Many concurrent operations
- Web handlers

**Use Spawn:**
- CPU-intensive work
- Blocking I/O
- True parallelism needed
- Isolated tasks

---

## Testing

### Unit Tests

```windjammer
fn fetch_user(id: int) -> User { }

#[test]
fn test_fetch_user() {
    // Just call it sync - easy!
    let user = fetch_user(1)
    assert_eq!(user.name, "Alice")
}
```

### Async Tests

```windjammer
#[test]
async fn test_fetch_user_concurrent() {
    async {
        let u1 = fetch_user(1)
        let u2 = fetch_user(2)
        assert_eq!(u1.id, 1)
        assert_eq!(u2.id, 2)
    }
}
```

### Parallel Tests

```windjammer
#[test]
fn test_parallel_processing() {
    let handles = vec![]
    for i in 1..10 {
        let handle = spawn process_item(i)
        handles.push(handle)
    }
    
    assert_eq!(handles.len(), 9)
}
```

---

## Error Handling

### Sync Errors

```windjammer
fn fetch() -> Result<Data> { }

match fetch() {
    Ok(data) => println!("Got: {}", data),
    Err(e) => println!("Error: {}", e),
}
```

### Async Errors

```windjammer
match async fetch() {
    Ok(future) => {
        match future.await {
            Ok(data) => println!("Got: {}", data),
            Err(e) => println!("Error: {}", e),
        }
    },
    Err(e) => println!("Spawn error: {}", e),
}

// Or simpler with async block:
async {
    match fetch() {
        Ok(data) => println!("Got: {}", data),
        Err(e) => println!("Error: {}", e),
    }
}
```

---

## Debugging

### Stack Traces

**Sync:**
```
main
└─ fetch_user
   └─ database.query
```

**Async:**
```
tokio::runtime
└─ async block
   └─ fetch_user (awaited)
      └─ database.query
```

**Spawn:**
```
thread::spawn
└─ fetch_user
   └─ database.query
```

### Debugging Tips

1. **Use `RUST_BACKTRACE=1`** to see full async traces
2. **Add logging** at function entry/exit
3. **Use `tokio-console`** for async runtime inspection
4. **Search for `async` and `spawn`** keywords to find concurrency points

---

## Future Enhancements

### Phase 2: Structured Concurrency

```windjammer
// Automatic cancellation and error propagation
async {
    let r1 = fetch1()  // If this fails...
    let r2 = fetch2()  // ...this is cancelled
}
```

### Phase 3: Async Iterators

```windjammer
async {
    for item in stream.items() {
        process(item)  // Each iteration awaited
    }
}
```

### Phase 4: Select/Race

```windjammer
async {
    let result = select {
        r1 = fetch1() => r1,
        r2 = fetch2() => r2,
    }  // First to complete wins
}
```

---

## References

- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Go Concurrency Patterns](https://go.dev/blog/concurrency-patterns)
- [Tokio Documentation](https://tokio.rs/tokio/tutorial)
- Windjammer Philosophy: Caller-controlled, zero crate leakage

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-11-03 | Adopt call-site explicit async | Best balance of type safety and ergonomics |
| 2025-11-03 | Implicit await in async blocks | Reduces boilerplate for common case |
| 2025-11-03 | Decorators create async context | Web handlers are inherently async |
| 2025-11-03 | Support both prefix and blocks | Flexibility for different use cases |

---

**Status:** Design Complete - Ready for Implementation  
**Target Version:** v2.0 (Breaking Change)  
**Implementation Estimate:** 2-3 weeks  
**Migration Tool:** Required

