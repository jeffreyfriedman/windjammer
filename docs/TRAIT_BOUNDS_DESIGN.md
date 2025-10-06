# Trait Bounds in Windjammer: The 80/20 Approach

**Status**: Implemented in v0.7.0  
**Philosophy**: Rust's power with cleaner syntax

---

## The Problem with Rust

Rust's trait bounds are **powerful but verbose**:

```rust
// Rust: Repetitive and hard to read
fn process<T: Display + Clone + Send + Sync>(item: T) -> String 
where 
    T: Hash + Eq,
{
    format!("{}", item)
}

// Common patterns repeated everywhere
fn show<T: Display + Debug>(x: T) { }
fn compare<T: PartialEq + Eq + Hash>(x: T, y: T) { }
fn thread_safe<T: Send + Sync + Clone>(x: T) { }
```

**Problems:**
1. ❌ Verbose: `Display + Clone + Send + Sync` repeated constantly
2. ❌ Cluttered: Bounds mixed with generics in signature
3. ❌ Not DRY: Same combinations copy-pasted everywhere
4. ❌ Hard to change: Update one function = update many others

---

## Windjammer's 80/20 Solution

### **Level 1: Direct Bounds (Rust Compatible)**

For simple cases, use Rust's syntax (works immediately):

```windjammer
// Simple inline bound
fn show<T: Display>(item: T) {
    println!("{}", item)
}

// Multiple bounds with +
fn process<T: Display + Clone>(item: T) {
    let copy = item.clone()
    println!("{}", copy)
}

// Where clauses for complex constraints
fn advanced<T, U>(a: T, b: U) 
where 
    T: Display + Clone,
    U: Debug + Send
{
    // ...
}
```

**The Win:** Full Rust compatibility, works immediately, no learning curve.

### **Level 2: Type Aliases for Bounds (80% Win!)**

Define common trait combinations **once**, use **everywhere**:

```windjammer
// Define trait aliases (like type aliases but for traits)
type Printable = Display + Debug
type Comparable = Clone + Eq + Hash  
type Threadsafe = Send + Sync

// Now use them!
fn show<T: Printable>(item: T) {
    println!("{:?}", item)
}

fn compare<T: Comparable>(a: T, b: T) -> bool {
    a == b
}

fn spawn_task<T: Threadsafe>(data: T) {
    go {
        // Safe to send across threads
        process(data)
    }
}
```

**The Wins:**
- ✅ **DRY**: Define once, use everywhere
- ✅ **Self-documenting**: "Printable" > "Display + Debug"
- ✅ **Easy to change**: Update alias, all uses update
- ✅ **Team conventions**: Shared vocabulary
- ✅ **80% less typing**: Most functions use 2-3 common patterns

### **Level 3: Bound Inference (Future: v0.8.0)**

Let the compiler **infer bounds from usage**:

```windjammer
// v0.8.0 goal: No bounds needed!
fn process<T>(item: T) {
    println!("{}", item)    // Compiler infers: T: Display
    let x = item.clone()     // Compiler infers: T: Clone
    send_to_thread(item)     // Compiler infers: T: Send
}
// Generated: fn process<T: Display + Clone + Send>(item: T)
```

**Why Later?**
- Need more sophisticated analysis
- Want to ensure good error messages
- Phase 1 & 2 deliver 80% of the benefit now

---

## Real-World Examples

### **Example 1: API Handler**

**Rust (verbose):**
```rust
fn handle_request<T>(body: T) -> Response 
where 
    T: Serialize + Deserialize + Clone + Send + Sync
{
    // ... lots of functions with same bounds
}

fn validate<T>(data: T) -> bool 
where 
    T: Serialize + Deserialize + Clone + Send + Sync
{
    // ...
}

fn store<T>(data: T) 
where 
    T: Serialize + Deserialize + Clone + Send + Sync
{
    // ...
}
```

**Windjammer (clean):**
```windjammer
// Define once at the top of your module
type ApiData = Serialize + Deserialize + Clone + Threadsafe

// Use everywhere - clear and concise
fn handle_request<T: ApiData>(body: T) -> Response {
    validate(body.clone())
    store(body)
}

fn validate<T: ApiData>(data: T) -> bool {
    // ...
}

fn store<T: ApiData>(data: T) {
    // ...
}
```

**Impact:** 
- 5 functions × 40 chars = **200 characters → 40 characters** (80% reduction!)
- Change requirements? Update **one line** instead of five
- New developer? Immediately understands "ApiData"

### **Example 2: Collections Library**

**Rust:**
```rust
fn deduplicate<T: Clone + Eq + Hash>(items: Vec<T>) -> Vec<T> { }
fn sort_unique<T: Clone + Eq + Hash + Ord>(items: Vec<T>) -> Vec<T> { }
fn find_duplicates<T: Clone + Eq + Hash>(items: Vec<T>) -> Vec<T> { }
fn intersection<T: Clone + Eq + Hash>(a: Vec<T>, b: Vec<T>) -> Vec<T> { }
```

**Windjammer:**
```windjammer
type Hashable = Clone + Eq + Hash
type Sortable = Hashable + Ord

fn deduplicate<T: Hashable>(items: Vec<T>) -> Vec<T> { }
fn sort_unique<T: Sortable>(items: Vec<T>) -> Vec<T> { }
fn find_duplicates<T: Hashable>(items: Vec<T>) -> Vec<T> { }
fn intersection<T: Hashable>(a: Vec<T>, b: Vec<T>) -> Vec<T> { }
```

**Impact:**
- Immediately clear what each function needs
- `Hashable` tells you more than `Clone + Eq + Hash`
- Easy to add `Debug` to all: change one line

---

## Implementation Details

### **Type Alias Syntax**

```windjammer
// Simple alias
type Printable = Display + Debug

// Complex combinations
type WebHandler = Serialize + Deserialize + Send + Sync + Clone

// Nested (uses existing aliases)
type AsyncHandler = WebHandler + Future
```

### **Where Clauses**

For complex constraints:

```windjammer
fn complex<T, U>(a: T, b: U) -> Result<T, Error>
where
    T: Display + Clone,
    U: Into<T> + Debug,
    T::Output: Send
{
    // ...
}
```

### **Associated Type Bounds**

```windjammer
fn process<T>(iter: T)
where
    T: Iterator,
    T::Item: Display
{
    for item in iter {
        println!("{}", item)
    }
}
```

---

## Comparison: Before & After

### **Scenario: Building a Web API**

**Rust (repetitive):**
```rust
// Repeated 10+ times in your codebase
fn create_user<T: Serialize + Deserialize + Clone + Send>(data: T) { }
fn update_user<T: Serialize + Deserialize + Clone + Send>(data: T) { }
fn delete_user<T: Serialize + Deserialize + Clone + Send>(data: T) { }
// ... etc
```

**Windjammer (once & done):**
```windjammer
// Define once
type RequestBody = Serialize + Deserialize + Clone + Send

// Use everywhere
fn create_user<T: RequestBody>(data: T) { }
fn update_user<T: RequestBody>(data: T) { }
fn delete_user<T: RequestBody>(data: T) { }
```

---

## Benefits Summary

| Feature | Rust | Windjammer | Win |
|---------|------|------------|-----|
| **Simple bounds** | `T: Display` | `T: Display` | Same ✓ |
| **Multiple bounds** | `T: Display + Clone + Send` | `T: Display + Clone + Send` | Same ✓ |
| **Repeated patterns** | Copy-paste everywhere | `type Printable = ...` | **80% less code** ✨ |
| **Readability** | Implementation details | Business meaning | **Clearer intent** ✨ |
| **Maintainability** | Change in N places | Change in 1 place | **DRY** ✨ |
| **Team conventions** | Informal | Explicit aliases | **Shared vocabulary** ✨ |

---

## Migration Path

### **Phase 1: v0.7.0 (Now)**
✅ Parse trait bounds: `T: Display`  
✅ Parse where clauses  
✅ Type aliases for traits  
✅ Generate correct Rust code  

### **Phase 2: v0.8.0 (Future)**
🔮 Infer bounds from function body  
🔮 Suggest type aliases for repeated patterns  
🔮 Auto-complete bound aliases  

### **Phase 3: v0.9.0+ (Future)**
🔮 Bound relaxation warnings  
🔮 Minimal bound suggestions  
🔮 Cross-crate bound analysis  

---

## Design Rationale

### **Why Type Aliases, Not Supertrait?**

Rust has `trait Printable: Display + Debug { }` but:
- ❌ More ceremony (trait declaration)
- ❌ Requires explicit `impl Printable for T`
- ❌ Doesn't compose well

Type aliases are:
- ✅ Lighter weight (one line)
- ✅ Automatic (no impl needed)
- ✅ Compose naturally

### **Why Not Full Inference Now?**

Inference is **powerful but complex**:
- Needs sophisticated analysis
- Error messages must be clear
- Type aliases deliver 80% benefit immediately
- Can add inference later without breaking changes

### **Comparison to Other Languages**

| Language | Approach | Windjammer |
|----------|----------|------------|
| **Rust** | Explicit bounds | ✅ Compatible |
| **Haskell** | Type classes | Similar to aliases |
| **TypeScript** | Intersection types | Inspired our aliases |
| **Go** | Implicit (structural) | Too loose for systems |
| **Swift** | Protocol composition | Similar philosophy |

---

## Usage Guidelines

### **When to Use Type Aliases**

✅ **DO use aliases for:**
- Repeated trait combinations (3+ uses)
- Complex bounds (3+ traits)
- Domain concepts ("WebHandler", "ApiData")
- Team conventions

❌ **DON'T use aliases for:**
- One-off simple bounds (`T: Display`)
- Standard library types with obvious meaning
- Over-abstraction (YAGNI)

### **Naming Conventions**

```windjammer
// ✅ GOOD: Describes capability or role
type Printable = Display + Debug
type Comparable = Clone + Eq + Hash
type WebHandler = Serialize + Deserialize + Send

// ❌ BAD: Too vague or implementation-focused
type MyTrait = Display + Debug
type Stuff = Clone + Send
type Data = Serialize
```

---

## Future: Bound Inference

**v0.8.0 Goal**: Compiler figures out bounds from usage

```windjammer
// You write:
fn process<T>(item: T) {
    println!("{}", item)        // Uses Display
    let copy = item.clone()      // Uses Clone  
    send_across_thread(copy)     // Uses Send
}

// Compiler generates:
fn process<T: Display + Clone + Send>(item: T) {
    // ...
}
```

**Benefits:**
- Write what you mean, not the bureaucracy
- Bounds stay in sync with usage
- Better error messages (points to usage, not signature)

**Challenges:**
- Must generate good errors
- Must handle inference failures gracefully
- Must not confuse users

**Decision:** Deliver explicit bounds now (80% benefit), add inference later (20% benefit).

---

## Conclusion

Windjammer's trait bound system:

1. ✅ **Compatible**: Works like Rust today
2. ✅ **Cleaner**: Type aliases eliminate repetition (80% win)
3. ✅ **Evolvable**: Can add inference later without breaking changes
4. ✅ **Practical**: Solves real problems developers face daily

**The 80/20 philosophy in action**: Deliver the biggest ergonomic wins first, iterate on advanced features later.

---

*Last Updated: v0.7.0 - October 2025*
