# Decorator System Design

**Status**: Design Phase  
**Target Version**: 0.40.0+  
**Priority**: Medium (post-MVP)

---

## Problem Statement

Currently, the Windjammer compiler has hardcoded support for game-specific decorators (`@game`, `@update`, `@render`, etc.) in the code generator. This violates separation of concerns and prevents library authors from defining their own decorators.

**Key Issues**:
1. **Tight Coupling**: Compiler knows about `windjammer-game` library
2. **Not Extensible**: Library authors can't define custom decorators
3. **Language Lock-in**: Must use Rust proc macros (doesn't work for Windjammer libraries)
4. **Maintenance Burden**: Game engine changes require compiler changes

---

## Design Goals

### 1. **Separation of Concerns**
- Compiler should not know about specific libraries or frameworks
- Decorators should be defined by libraries, not the compiler
- Core language remains general-purpose

### 2. **Windjammer-Native**
- Library authors write decorators in Windjammer, not Rust
- Works across all compilation targets (Rust, JS, WASM, etc.)
- No dependency on Rust proc macros

### 3. **Compile-Time Execution**
- Decorators run during compilation
- Can generate code, modify AST, or validate usage
- Type-safe and predictable

### 4. **Composable**
- Multiple decorators can be applied to the same item
- Decorators can be chained or nested
- Clear execution order

---

## Proposed Architecture

### Option 1: Compile-Time Functions (Recommended)

**Concept**: Decorators are Windjammer functions that run at compile time and transform AST nodes.

```windjammer
// In windjammer-game/decorators/game.wj

@compile_time
pub fn game_decorator(target: StructDecl) -> Vec<Item> {
    // Validate the struct
    if !has_default_impl(target) {
        compile_error("@game struct must implement Default")
    }
    
    // Generate GameLoop implementation
    let game_loop_impl = generate_game_loop_impl(target.name)
    
    // Generate main function
    let main_fn = generate_main_function(target.name)
    
    // Return additional items to inject
    vec![game_loop_impl, main_fn]
}

// Register the decorator
@decorator("game")
pub fn game(target: StructDecl) -> Vec<Item> {
    game_decorator(target)
}
```

**Usage**:
```windjammer
use windjammer_game::decorators::*;

@game
struct MyGame {
    score: int,
}

// Compiler automatically generates:
// - impl GameLoop for MyGame { ... }
// - fn main() { ... }
```

**Pros**:
- ✅ Pure Windjammer (no Rust proc macros)
- ✅ Type-safe AST manipulation
- ✅ Works across all targets
- ✅ Easy to debug (just Windjammer code)

**Cons**:
- ❌ Requires compile-time execution engine
- ❌ More complex compiler implementation

---

### Option 2: Template-Based Decorators (Simpler)

**Concept**: Decorators are templates that get expanded with substitutions.

```windjammer
// In windjammer-game/decorators/game.wj

@decorator
template game(struct_name: Ident) {
    impl GameLoop for {{struct_name}} {
        fn init(&mut self) {
            // Default implementation
        }
        
        fn update(&mut self, delta: f32) {
            // User overrides this
        }
        
        fn render(&mut self, ctx: &mut RenderContext) {
            // User overrides this
        }
    }
    
    fn main() {
        let mut game = {{struct_name}}::default();
        GameApp::new("{{struct_name}}", 800, 600)
            .run(game)
            .expect("Failed to run game");
    }
}
```

**Pros**:
- ✅ Simpler to implement
- ✅ Familiar template syntax
- ✅ No compile-time execution needed

**Cons**:
- ❌ Less powerful (just text substitution)
- ❌ Can't validate or check types
- ❌ Limited control flow

---

### Option 3: Macro System (Most Powerful)

**Concept**: Full macro system like Rust's `macro_rules!` or Lisp macros.

```windjammer
// In windjammer-game/macros/game.wj

macro game($struct_name:ident) {
    impl GameLoop for $struct_name {
        fn update(&mut self, delta: f32) {
            // Generated code
        }
        
        fn render(&mut self, ctx: &mut RenderContext) {
            // Generated code
        }
    }
    
    fn main() {
        let mut game = $struct_name::default();
        GameApp::new(stringify!($struct_name), 800, 600)
            .run(game)
            .expect("Failed to run game");
    }
}
```

**Pros**:
- ✅ Most powerful and flexible
- ✅ Industry-proven approach (Rust, Lisp)
- ✅ Can handle complex transformations

**Cons**:
- ❌ Most complex to implement
- ❌ Steeper learning curve for users
- ❌ Can be hard to debug

---

## Recommended Approach

**Phase 1** (v0.40): **Template-Based Decorators**
- Simplest to implement
- Covers 80% of use cases
- Gets us unblocked quickly

**Phase 2** (v0.45): **Compile-Time Functions**
- More powerful for complex transformations
- Type-safe AST manipulation
- Better error messages

**Phase 3** (v0.50+): **Full Macro System** (if needed)
- Only if template + compile-time functions aren't enough
- Learn from user feedback first

---

## Implementation Plan

### Phase 1: Template-Based Decorators

**1. Decorator Registration** (in stdlib or compiler)
```windjammer
// std/decorator.wj
pub fn register_decorator(name: string, template: Template) { ... }
```

**2. Template Parser**
- Parse `@decorator template name(...) { ... }` syntax
- Extract template parameters
- Store template in decorator registry

**3. Template Expansion**
- When `@name` is encountered, look up template
- Substitute parameters
- Inject generated code into AST

**4. Compiler Integration**
- Add decorator expansion pass after parsing
- Before type checking and code generation
- Maintain source maps for error messages

### Phase 2: Compile-Time Functions

**1. Compile-Time Execution Engine**
- Interpret Windjammer code at compile time
- Provide AST manipulation APIs
- Sandbox for safety

**2. AST API**
```windjammer
// std/ast.wj
pub struct StructDecl {
    pub name: string,
    pub fields: Vec<Field>,
    pub decorators: Vec<Decorator>,
    // ...
}

pub fn generate_impl(type_name: string, trait_name: string, methods: Vec<Function>) -> Item { ... }
```

**3. Decorator Functions**
```windjammer
@compile_time
pub fn my_decorator(target: StructDecl) -> Vec<Item> {
    // Transform AST
}
```

---

## Migration Path

### Current State (v0.38.6)
- ❌ `@game` decorator hardcoded in compiler
- ❌ Tight coupling to `windjammer-game`
- ✅ Works, but not extensible

### Step 1: Remove `@game` Decorator (v0.38.6)
- ✅ Remove hardcoded game framework code
- ✅ Use explicit `GameApp` API instead
- ✅ Clean separation of concerns
- **Status**: In progress

### Step 2: Design Decorator System (v0.39-0.40)
- Document requirements and use cases
- Prototype template-based system
- Get community feedback

### Step 3: Implement Template Decorators (v0.40)
- Add decorator registration
- Implement template parser and expander
- Migrate `@game` to library-defined decorator

### Step 4: Implement Compile-Time Functions (v0.45)
- Add compile-time execution engine
- Provide AST manipulation APIs
- Enable more powerful transformations

---

## Use Cases

### 1. Game Framework (`@game`)
```windjammer
@game
struct MyGame { ... }

// Generates:
// - impl GameLoop for MyGame
// - fn main() with GameApp setup
```

### 2. Serialization (`@serialize`)
```windjammer
@serialize
struct Config {
    name: string,
    port: int,
}

// Generates:
// - impl Serialize for Config
// - impl Deserialize for Config
```

### 3. REST API (`@rest`)
```windjammer
@rest("/api/users")
struct UserController {
    @get("/:id")
    fn get_user(id: int) -> User { ... }
    
    @post("/")
    fn create_user(user: User) -> User { ... }
}

// Generates:
// - Route registration
// - Request/response handling
// - Error handling
```

### 4. Database ORM (`@table`)
```windjammer
@table("users")
struct User {
    @primary_key
    id: int,
    
    @unique
    email: string,
    
    name: string,
}

// Generates:
// - SQL schema
// - CRUD methods
// - Query builders
```

---

## Open Questions

1. **Decorator Composition**: How do multiple decorators interact?
   - Sequential application?
   - Dependency resolution?
   - Conflict detection?

2. **Error Handling**: How do decorators report errors?
   - Compile-time errors with source locations?
   - Warnings vs errors?
   - Suggestions for fixes?

3. **Debugging**: How do users debug decorator-generated code?
   - Show expanded code?
   - Step through decorator execution?
   - Source maps?

4. **Performance**: How do we keep compilation fast?
   - Cache expanded decorators?
   - Incremental compilation?
   - Parallel decorator execution?

5. **Security**: How do we sandbox decorator execution?
   - Limit file system access?
   - Prevent infinite loops?
   - Resource limits?

---

## References

### Similar Systems

1. **Rust Procedural Macros**
   - Powerful but Rust-specific
   - Requires separate crate
   - Good error messages

2. **Python Decorators**
   - Runtime, not compile-time
   - Simple syntax
   - Very flexible

3. **Java Annotations + Annotation Processors**
   - Compile-time code generation
   - Separate processor step
   - Good IDE integration

4. **C# Attributes + Source Generators**
   - Compile-time code generation
   - Roslyn API for AST manipulation
   - Excellent tooling

---

## Decision Log

### 2025-11-28: Removed `@game` Decorator
**Decision**: Remove hardcoded `@game` decorator from compiler  
**Rationale**: Violates separation of concerns, prevents extensibility  
**Impact**: Users must use explicit `GameApp` API (which is better anyway)  
**Next Steps**: Design proper decorator system for v0.40+

---

## Status

- [x] Document current problem
- [x] Propose architecture options
- [x] Define use cases
- [ ] Prototype template-based system
- [ ] Get community feedback
- [ ] Implement Phase 1 (templates)
- [ ] Implement Phase 2 (compile-time functions)

---

**Last Updated**: 2025-11-28  
**Author**: Windjammer Team  
**Version**: 0.1 (Draft)


