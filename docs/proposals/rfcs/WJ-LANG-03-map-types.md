# WJ-LANG-03: Map Types for Windjammer Standard Library

**Status**: Draft  
**Author**: Windjammer Team  
**Created**: 2026-03-28  
**Category**: Standard Library Design

## Summary

Windjammer currently has no `Map` type in its standard library. This RFC proposes a map type hierarchy that is backend-agnostic, game-development-friendly, and follows the Windjammer philosophy: "compiler does the hard work, not the developer."

## Problem Statement

Windjammer `.wj` files currently fall back to Rust-specific types like `std::collections::HashMap` or use `Vec` as a workaround. This violates backend-agnosticism (Go and JS have their own map types) and exposes implementation details.

## Research: Existing Map Implementations

### Standard HashMap (Rust: `std::collections::HashMap`)
- **Use case**: General-purpose key-value storage
- **Performance**: O(1) amortized insert/lookup, O(n) worst case
- **Thread safety**: None (single-threaded only)
- **Backend mapping**: Rust `HashMap`, Go `map[K]V`, JS `Map`

### SlotMap (`slotmap` crate)
- **Use case**: Entity storage in game engines (stable keys, generational indices)
- **Performance**: O(1) insert/remove/access, stored in `Vec` internally
- **Key feature**: Keys are versioned - removed keys are forever invalid
- **Why it matters**: Perfect for ECS, game entities, graph nodes
- **Variants**: `SlotMap` (fast access, slow iteration over sparse data), `HopSlotMap` (faster iteration), `DenseSlotMap` (fastest iteration, slower access)
- **Thread safety**: None (single-threaded)

### DashMap (`dashmap` crate)
- **Use case**: Concurrent HashMap for multi-threaded access
- **Performance**: Uses sharded locking for parallelism
- **Key feature**: `&self` for all operations (can be shared via `Arc`)
- **Thread safety**: Full concurrent read/write
- **Caveat**: Can deadlock if you hold references across operations

### Papaya (`papaya` crate)
- **Use case**: Read-heavy concurrent HashMap
- **Performance**: Lock-free reads, near-linear read scaling
- **Key feature**: Extremely fast reads, predictable latency
- **Thread safety**: Lock-free reads, incremental resizing
- **Best for**: Config caches, asset registries, resource lookups

### Flashmap (`flashmap` crate)
- **Use case**: Single-writer, multi-reader concurrent HashMap
- **Performance**: Wait-free reads on x86, single-writer
- **Key feature**: Readers see snapshots, eventual consistency
- **Thread safety**: Single writer + many readers
- **Best for**: Game state broadcasting, config distribution

### Crossbeam utilities
- **Use case**: Concurrent programming primitives
- **Key features**: Epoch-based GC, channels, work-stealing deques
- **Relevance**: Foundation for building concurrent data structures

## Proposed Windjammer Map Types

### Tier 1: Core (Must have)

#### `Map<K, V>` - General Purpose Map
The default map type. Backend-agnostic general-purpose key-value storage.

```windjammer
let mut scores = Map::new()
scores.insert("Alice", 100)
scores.insert("Bob", 85)

let alice_score = scores.get("Alice")  // Option<i32>

for (name, score) in scores {
    println!("{}: {}", name, score)
}
```

**Backend mapping**:
- Rust: `HashMap<K, V>` (from `std::collections`)
- Go: `map[K]V`
- JavaScript: `new Map()`
- Interpreter: Internal hash table

**Methods**:
- `new()` → `Map<K, V>`
- `insert(key, value)` → `Option<V>` (returns old value)
- `get(key)` → `Option<V>`
- `remove(key)` → `Option<V>`
- `contains_key(key)` → `bool`
- `len()` → `i32`
- `is_empty()` → `bool`
- `keys()` → iterable
- `values()` → iterable
- `clear()`

### Tier 2: Game Development (Should have)

#### `SlotMap<V>` - Generational Index Map
Stable keys for game entities, scene nodes, resources. Keys are never reused incorrectly.

```windjammer
let mut entities = SlotMap::new()
let player = entities.insert(Player::new("Hero"))
let enemy = entities.insert(Enemy::new("Goblin"))

entities.remove(enemy)

// Key is forever invalid after removal - no ABA problem
let result = entities.get(enemy)  // None

// Safe iteration over active entities only
for (key, entity) in entities {
    entity.update(dt)
}
```

**Why SlotMap is critical for games**:
- Entities are created and destroyed frequently
- Other systems hold references (keys) to entities
- With Vec/HashMap, removing entity `i` shifts indices or leaves gaps
- SlotMap keys include a generation counter - removed keys stay invalid forever
- O(1) insert, remove, and access

**Backend mapping**:
- Rust: `slotmap::SlotMap` (external crate)
- Go/JS: Custom implementation (generational index array)
- Interpreter: Custom implementation

#### `OrderedMap<K, V>` - Insertion-Order Preserving Map
Iterates in insertion order. Useful for configs, UI layouts, serialization.

```windjammer
let mut config = OrderedMap::new()
config.insert("width", 1920)
config.insert("height", 1080)
config.insert("fullscreen", 1)

// Iterates in insertion order
for (key, value) in config {
    println!("{} = {}", key, value)
}
```

**Backend mapping**:
- Rust: `indexmap::IndexMap`
- Go: Custom (linked list + map)
- JavaScript: `Map` (already ordered)

### Tier 3: Concurrent (Nice to have)

#### `ConcurrentMap<K, V>` - Thread-Safe Map
For game systems that need shared state across threads (e.g., asset cache, entity registry).

```windjammer
let assets = ConcurrentMap::new()

// Multiple threads can read/write simultaneously
spawn || {
    assets.insert("texture_1", load_texture("player.png"))
}

spawn || {
    if let Some(tex) = assets.get("texture_1") {
        render(tex)
    }
}
```

**Backend mapping**:
- Rust: `dashmap::DashMap` (sharded locks) or `papaya::HashMap` (lock-free reads)
- Go: `sync.Map` or external concurrent map
- JavaScript: Not needed (single-threaded + event loop)

**Design decision**: Use `papaya` under the hood for read-heavy workloads (common in games: asset lookup is frequent, asset loading is rare). Fall back to `dashmap` for write-heavy workloads.

## Design Principles

### 1. Backend-Agnostic API
All map types have the same API surface regardless of backend. The compiler maps to the most efficient native implementation.

### 2. Inference over Annotation
```windjammer
// Type inferred from usage:
let mut m = Map::new()
m.insert("key", 42)  // Compiler infers Map<String, i32>
```

### 3. Auto-Derive
All map types automatically derive `Debug`, `Clone`, `PartialEq` when their key/value types support it. No manual `#[derive(...)]` needed.

### 4. Iteration is Natural
```windjammer
for (key, value) in map {
    // Compiler handles & vs owned based on usage
}
```

### 5. No Rust Leakage
- No `&` in map methods (compiler infers borrowing)
- No `.unwrap()` (use pattern matching or `?`)
- No `HashMap::new()` (use `Map::new()`)
- No `use std::collections::HashMap` (use `use std::map::Map`)

## Implementation Plan

### Phase 1: `Map<K, V>` (Core)
1. Add `Map` type to Windjammer parser (recognized type name)
2. Add codegen mapping: `Map<K, V>` → `HashMap<K, V>` (Rust), `map[K]V` (Go), `Map` (JS)
3. Add stdlib method declarations for `insert`, `get`, `remove`, etc.
4. Add auto-derive support
5. TDD: Write tests in `.wj` files

### Phase 2: `SlotMap<V>` (Game Dev)
1. Add `SlotMap` type recognition
2. Add `slotmap` as optional dependency in generated Cargo.toml
3. Codegen mapping for Rust backend
4. Custom implementation for Go/JS backends
5. TDD: Entity lifecycle tests

### Phase 3: `OrderedMap<K, V>` and `ConcurrentMap<K, V>`
1. Add type recognition and codegen
2. Backend-specific implementations
3. TDD: Ordering and concurrency tests

## Comparison with Other Languages

| Feature | Rust | Go | JavaScript | Windjammer |
|---|---|---|---|---|
| Default map | `HashMap` | `map` | `Map`/`Object` | `Map` |
| Ordered map | `IndexMap` | N/A | `Map` (built-in) | `OrderedMap` |
| Concurrent map | `DashMap`/`papaya` | `sync.Map` | N/A | `ConcurrentMap` |
| Entity map | `slotmap` | Custom | Custom | `SlotMap` |
| Explicit `&` | Required | N/A | N/A | Inferred |
| Type annotation | Required | Inferred | Dynamic | Inferred |

## Decision Record

### Why not just one `Map` type?
Different use cases have fundamentally different performance characteristics:
- General maps: amortized O(1), good enough for most cases
- SlotMaps: stable keys critical for game entity systems
- Concurrent maps: thread safety critical for parallel game systems

### Why not expose all Rust crate types?
Exposing `DashMap`, `papaya`, `flashmap` etc. directly would tie Windjammer to Rust's ecosystem. Our types are backend-agnostic wrappers that compile to the best available implementation for each backend.

### Why SlotMap is a separate type (not a Map variant)?
SlotMap has a fundamentally different key model (generated keys vs user-provided keys). Trying to unify them under one API would be confusing and lose type safety. A developer choosing `SlotMap` is making a deliberate architectural decision about entity management.
