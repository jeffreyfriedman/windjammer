# Windjammer Compiler Architecture

## Overview

Windjammer is a systems programming language that transpiles to Rust, designed to provide 80% of Rust's power with 20% of its complexity.

## Crate Structure

```
windjammer/
├── windjammer (main compiler)
│   ├── Parser
│   ├── Type inference
│   ├── Code generation
│   └── component/ (optional, feature-gated)
│       └── UI component compilation
│
└── crates/
    ├── windjammer-runtime      (stdlib implementations)
    ├── windjammer-ui           (UI framework runtime)
    ├── windjammer-ui-macro     (procedural macros)
    ├── windjammer-lsp          (Language Server Protocol)
    ├── windjammer-mcp          (Model Context Protocol)
    └── windjammer-game-framework (game engine)
```

## Dependency Graph

### Core Dependencies

```
┌─────────────────────────────────────────────────────────────┐
│                      windjammer (compiler)                   │
│                                                              │
│  Core:                                                       │
│  - Parser                                                    │
│  - Type inference                                            │
│  - Code generation                                           │
│                                                              │
│  Optional (feature = "ui-components"):                       │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ component/                                              │ │
│  │  - Parse .wj component files                           │ │
│  │  - Analyze reactive dependencies                       │ │
│  │  - Transform to signals                                │ │
│  │  - Generate component code                             │ │
│  └────────────────────────────────────────────────────────┘ │
└────────────────────────┬─────────────────────────────────────┘
                         │ optional dependency
                         │ (feature = "ui-components")
                         ▼
         ┌───────────────────────────────┐
         │     windjammer-ui             │
         │     (runtime only)            │
         │                               │
         │  - Signal<T>                  │
         │  - Computed<T>                │
         │  - Effect                     │
         │  - Virtual DOM                │
         │  - Platform abstraction       │
         └───────────────────────────────┘
                         ▲
                         │ NO dependencies
                         │ (pure runtime)
```

### Key Principle: No Circular Dependencies

**Decision**: The compiler can optionally depend on the UI runtime, but the runtime NEVER depends on the compiler.

**Rationale**:
1. **Runtime is standalone** - Can be used without the compiler for hand-written components
2. **Clean separation** - Compile-time vs. runtime concerns are separate
3. **Smaller binaries** - Users can use runtime without compiler overhead
4. **Easier testing** - Runtime can be tested independently

**Implementation**:
```toml
# Cargo.toml
[features]
default = []
ui-components = ["windjammer-ui"]

[dependencies]
windjammer-ui = { path = "crates/windjammer-ui", optional = true }
```

```rust
// src/main.rs
#[cfg(feature = "ui-components")]
pub mod component;
```

### Why Not Other Approaches?

#### ❌ Component Compiler in Main Compiler (Always)
```
windjammer → windjammer-ui (always)
```
**Problem**: Tight coupling, can't use UI without full compiler

#### ❌ Component Compiler in UI Crate
```
windjammer ↔ windjammer-ui (circular!)
```
**Problem**: Circular dependency, runtime depends on compiler

#### ❌ Separate Component Compiler Crate
```
windjammer → windjammer-component-compiler → windjammer-ui
```
**Problem**: More crates to manage, potential code duplication

#### ✅ Feature-Gated in Main Compiler (Chosen)
```
windjammer --[feature="ui-components"]--> windjammer-ui
```
**Benefits**: 
- Clean one-way dependency
- Optional compilation
- Code reuse (shared parser)
- Single crate to manage

---

## UI Component Architecture

### Design Philosophy

**Goal**: 80% of React/Svelte's power with 20% of the complexity

**Strategy**: Progressive disclosure
- **Simple components**: Minimal syntax (no boilerplate)
- **Complex components**: JSX-style with full control

### Component Syntax

#### Primary: Minimal Syntax (for 80% of cases)

```windjammer
// counter.wj - Simple component

// State (compiler makes reactive automatically)
count: int = 0

// Computed values
@computed
doubled: int = count * 2

// Functions
fn increment() {
    count += 1
}

// View (declarative)
view {
    div {
        p { "Count: {count}" }
        p { "Doubled: {doubled}" }
        button(on_click: increment) { "+" }
    }
}
```

**Features**:
- ✅ No boilerplate (no struct, no impl, no @component)
- ✅ Compiler infers reactivity automatically
- ✅ Pure Windjammer syntax (no HTML/XML)
- ✅ Declarative view definition
- ✅ Type-safe

**Compilation**:
1. Parse top-level declarations
2. Identify reactive variables
3. Transform to `Signal<T>`
4. Generate component struct
5. Generate render function

#### Advanced: JSX-Style (for 20% of cases)

```windjammer
// todo_list.wj - Complex component

@component
struct TodoList {
    todos: Vec<Todo>,
    filter: Filter,
}

impl TodoList {
    @computed
    fn filtered_todos(&self) -> Vec<&Todo> {
        self.todos.iter()
            .filter(|t| self.filter.matches(t))
            .collect()
    }
    
    fn render(&self) -> View {
        view! {
            <div class="todo-list">
                <FilterBar filter={self.filter} />
                {#each self.filtered_todos() as todo}
                    <TodoItem todo={todo} on:delete={self.delete} />
                {/each}
            </div>
        }
    }
}
```

**Features**:
- ✅ Explicit control (manual struct definition)
- ✅ Familiar to React developers
- ✅ Type-safe (macro validates at compile time)
- ✅ Composable (can use in any expression)

**Use Cases**:
- Complex state management
- Multiple lifecycle hooks
- Props and composition
- Advanced patterns

### State Management: Hybrid Approach

**User writes**: Simple reactive variables
```windjammer
count: int = 0
```

**Compiler generates**: Signal-based reactivity
```rust
struct Counter {
    count: Signal<i32>,
}
```

**Benefits**:
- ✅ Simple syntax for users
- ✅ Powerful runtime (fine-grained reactivity)
- ✅ Automatic dependency tracking
- ✅ Optimal performance

**Advanced users can use signals directly**:
```windjammer
count: Signal<int> = signal(0)  // Explicit signal

fn increment() {
    count.set(count.get() + 1)  // Manual control
}
```

---

## Compilation Pipeline

### Standard Windjammer Compilation

```
.wj file
   ↓
[Lexer] → Tokens
   ↓
[Parser] → AST
   ↓
[Type Inference] → Typed AST
   ↓
[Optimizer] → Optimized AST
   ↓
[Codegen] → Rust code
   ↓
rustc → Binary/WASM
```

### UI Component Compilation (feature = "ui-components")

```
.wj component file
   ↓
[Component Parser] → Parse state, computed, functions, view
   ↓
[Dependency Analyzer] → Track reactive dependencies
   ↓
[Signal Transformer] → Transform reactive vars → Signal<T>
   ↓
[Template Compiler] → Compile view { } → DOM operations
   ↓
[Component Codegen] → Generate component struct + impl
   ↓
[Standard Pipeline] → Rust code → Binary/WASM
```

---

## Module Organization

### Main Compiler (`src/`)

```
src/
├── main.rs              # Entry point, CLI
├── parser.rs            # Core parser
├── lexer.rs             # Lexer
├── analyzer.rs          # Type inference
├── codegen/             # Code generation
│   ├── mod.rs
│   ├── rust.rs          # Rust backend
│   ├── javascript.rs    # JavaScript backend
│   └── wasm.rs          # WASM backend
│
└── component/           # UI component compilation (optional)
    ├── mod.rs           # Public API
    ├── ast.rs           # Component AST
    ├── parser.rs        # Parse .wj components
    ├── analyzer.rs      # Dependency analysis
    ├── transformer.rs   # Reactive var → Signal
    ├── template.rs      # Template compilation
    └── codegen.rs       # Component code generation
```

### UI Runtime (`crates/windjammer-ui/src/`)

```
crates/windjammer-ui/src/
├── lib.rs               # Public API
├── reactivity.rs        # Signal, Computed, Effect
├── vdom.rs              # Virtual DOM
├── component.rs         # Component trait
├── platform/            # Platform abstraction
│   ├── web.rs           # Web (WASM)
│   ├── desktop.rs       # Desktop (Tauri)
│   └── mobile.rs        # Mobile
└── renderer.rs          # Rendering
```

---

## Design Decisions

### 1. Feature-Gated Component Compilation

**Decision**: UI component compilation is behind `ui-components` feature flag

**Rationale**:
- Not all users need UI features
- Keeps default build small
- Clean separation of concerns
- Optional dependency on runtime

**Trade-offs**:
- ✅ Smaller default binary
- ✅ Faster compilation (when not needed)
- ⚠️ Users must enable feature explicitly

### 2. Minimal Syntax as Primary

**Decision**: Simple syntax without boilerplate is the primary way to write components

**Rationale**:
- Matches "20% complexity" mission
- Lowers barrier to entry
- Reduces cognitive load
- Most components are simple

**Trade-offs**:
- ✅ Simplest possible for common cases
- ✅ Less code to write
- ⚠️ Less explicit (more "magic")
- ⚠️ Less familiar to Rust developers

### 3. JSX as Escape Hatch

**Decision**: Provide JSX-style syntax for complex components

**Rationale**:
- Familiar to React developers
- Provides full control when needed
- Progressive disclosure of complexity
- Handles edge cases

**Trade-offs**:
- ✅ Powerful for complex cases
- ✅ Familiar syntax
- ⚠️ Requires proc macro
- ⚠️ More verbose

### 4. Signal-Based Reactivity

**Decision**: Use fine-grained signals for reactivity, not Virtual DOM diffing

**Rationale**:
- Better performance (no full tree diff)
- Simpler mental model (direct updates)
- Proven pattern (Solid.js, Vue 3, Leptos)
- Composable primitives

**Trade-offs**:
- ✅ Excellent performance
- ✅ Fine-grained updates
- ⚠️ Requires automatic dependency tracking
- ⚠️ More complex compiler

### 5. Hybrid State Management

**Decision**: Users write simple variables, compiler generates signals

**Rationale**:
- Best ergonomics (simple syntax)
- Best performance (signal runtime)
- Progressive disclosure (can use signals explicitly)
- Matches mission (80/20)

**Trade-offs**:
- ✅ Simple for beginners
- ✅ Powerful for experts
- ⚠️ Compiler must infer reactivity
- ⚠️ Two mental models (simple vs. explicit)

---

## Future Considerations

### Potential Additions

1. **Server-Side Rendering (SSR)**
   - Render components to HTML on server
   - Hydration on client
   - SEO benefits

2. **Hot Module Replacement (HMR)**
   - Update components without full reload
   - Preserve state across updates
   - Better DX

3. **Component Library**
   - Standard components (Button, Input, etc.)
   - Accessible by default
   - Themeable

4. **DevTools**
   - Inspect component tree
   - Track signal updates
   - Performance profiling

### Potential Changes

1. **Separate Component Compiler Crate**
   - If component compilation becomes very complex
   - If other tools need to use it
   - If we want to version independently

2. **Multiple Syntax Styles**
   - Support both minimal and JSX simultaneously
   - Let users choose their preference
   - Provide migration tools

3. **Alternative Runtimes**
   - Different reactivity systems
   - Different rendering strategies
   - Platform-specific optimizations

---

## References

### Inspiration

- **Svelte**: Compiled reactivity, minimal syntax
- **Solid.js**: Fine-grained signals
- **React**: JSX, component model
- **Vue 3**: Hybrid reactivity (Proxy + signals)
- **Leptos**: Rust + signals + WASM

### Related Documents

- `docs/GUIDE.md` - User guide
- `docs/COMPARISON.md` - Comparison with other languages
- `ROADMAP.md` - Future plans
- `crates/windjammer-ui/README.md` - UI framework documentation

---

## Conclusion

This architecture provides:
- ✅ Clean separation between compiler and runtime
- ✅ No circular dependencies
- ✅ Progressive disclosure of complexity
- ✅ Simple syntax for common cases
- ✅ Powerful escape hatches for complex cases
- ✅ Production-quality reactivity system
- ✅ Matches Windjammer's mission (80% power, 20% complexity)

The feature-gated approach allows us to:
- Keep the default build small
- Provide optional UI features
- Maintain clean architecture
- Enable future growth

The minimal syntax with JSX escape hatch provides:
- Simple syntax for beginners
- Powerful tools for experts
- Familiar patterns for web developers
- Pure Windjammer (no HTML/XML in files)
