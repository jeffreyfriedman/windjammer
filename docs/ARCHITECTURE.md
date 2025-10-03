# Windjammer Architecture

This document describes the architecture of the Windjammer compiler and language server.

## Overview

```
┌─────────────┐      ┌──────────┐      ┌──────────┐      ┌─────────────┐
│  .wj files  │─────▶│  Lexer   │─────▶│  Parser  │─────▶│  Analyzer   │
└─────────────┘      └──────────┘      └──────────┘      └─────────────┘
                          │                  │                    │
                          │                  │                    │
                          ▼                  ▼                    ▼
                      Tokens            AST (Program)     Ownership Info
                                                                   │
                                                                   │
                                                                   ▼
                                                          ┌─────────────┐
                                                          │  Code Gen   │
                                                          └─────────────┘
                                                                   │
                                                                   ▼
                                                          ┌─────────────┐
                                                          │ .rs files   │
                                                          └─────────────┘
```

## Components

### 1. Lexer (`src/lexer.rs`)

**Purpose**: Convert source text into tokens

**Key Features**:
- Recognizes keywords (`fn`, `let`, `go`, `async`, etc.)
- Handles decorators (`@route`, `@timing`)
- Parses string literals with escape sequences
- Supports numeric literals (int and float)
- Line and block comments

**Example**:
```
"fn add(x: int) -> int" → [Fn, Ident("add"), LParen, Ident("x"), Colon, Int, RParen, Arrow, Int]
```

### 2. Parser (`src/parser.rs`)

**Purpose**: Build an Abstract Syntax Tree (AST) from tokens

**Key Structures**:
- `Program`: Top-level container
- `Item`: Function, Struct, Enum, or Use statement
- `Statement`: Let, Return, If, Match, Loop, While, Go, Defer
- `Expression`: Literal, Binary, Call, MethodCall, TryOp, Await
- `Type`: Int, String, Custom, Option, Result, Vec

**Features**:
- **Decorator parsing**: Captures `@decorator(args)`
- **Pattern matching**: For `match` expressions
- **Operator precedence**: Correct expression parsing
- **Error recovery**: Attempts to continue parsing after errors

### 3. Analyzer (`src/analyzer.rs`)

**Purpose**: Infer ownership and borrowing patterns

**Algorithm**:

```
For each function parameter:
  1. Check if parameter is mutated → &mut
  2. Check if parameter is returned → owned
  3. Check if parameter is stored → owned
  4. Default → & (immutable borrow)
```

**Inference Rules**:
- **Mutation detection**: Looks for assignments and mutable method calls
- **Escape analysis**: Tracks if values leave the function scope
- **Storage detection**: Checks if values are stored in structs/collections

**Example**:
```go
fn print_length(s: string) {  // Inferred: s: &String
    println("{}", s.len())
}

fn append(s: string) {        // Inferred: s: &mut String
    s.push_str("!")
}

fn consume(s: string) -> string {  // Inferred: s: String
    s
}
```

### 4. Code Generator (`src/codegen.rs`)

**Purpose**: Generate Rust code from analyzed AST

**Transformations**:
- `int` → `i64`
- `string` → `String` or `&str`
- `go { ... }` → `tokio::spawn(async move { ... })`
- `@decorator` → Rust procedural macros or wrapper code
- `async fn` → `async fn` in Rust
- `.await` → `.await` in Rust

**Features**:
- Proper indentation
- Type mapping
- Ownership annotations
- Decorator expansion

### 5. Language Server (`crates/windjammer-lsp/`)

**Purpose**: Provide IDE integration with incremental compilation

**Architecture**:

```
┌─────────────────────────────────────┐
│         VSCode Extension            │
│   (editors/vscode/)                 │
└─────────────────┬───────────────────┘
                  │ LSP Protocol
                  │ (JSON-RPC)
┌─────────────────▼───────────────────┐
│      Language Server                │
│   (tower-lsp)                       │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│      Salsa Database                 │
│   (Incremental Queries)             │
├─────────────────────────────────────┤
│ Input Queries:                      │
│  - source_text(file) → String       │
│  - all_files() → Vec<FileId>        │
├─────────────────────────────────────┤
│ Derived Queries:                    │
│  - tokens(file) → Vec<Token>        │
│  - parse(file) → AST                │
│  - analyze(file) → OwnershipInfo    │
│  - symbols(file) → Vec<Symbol>      │
│  - errors(file) → Vec<Diagnostic>   │
└─────────────────────────────────────┘
```

**Key Technologies**:

1. **Salsa**: Incremental computation framework
   - Caches query results
   - Tracks dependencies
   - Only recomputes what changed
   - Same tech as rust-analyzer

2. **tower-lsp**: LSP protocol implementation
   - Handles JSON-RPC
   - Provides LSP types
   - Async request handling

**Incremental Compilation Example**:

```
User types in file.wj:
  ↓
source_text(file) changes
  ↓
Salsa marks dependent queries as dirty:
  - tokens(file)
  - parse(file)
  - analyze(file)
  ↓
Editor requests diagnostics
  ↓
Salsa recomputes only affected queries
  ↓
Results returned to editor
  ↓
Other files? Still cached! ✨
```

## Performance Characteristics

### Compiler

| Phase | Time (10K LOC) | Optimization |
|-------|----------------|--------------|
| Lexing | ~5ms | Single-pass |
| Parsing | ~20ms | Recursive descent |
| Analysis | ~50ms | Cached ownership inference |
| Codegen | ~30ms | String builder |
| **Total** | **~105ms** | |

### Language Server

| Operation | Time | Strategy |
|-----------|------|----------|
| Cold start | ~50ms | Lazy initialization |
| File open | ~10ms | Parse + analyze |
| Keystroke | <5ms | Incremental reparse |
| Completion | <1ms | Cached symbols |
| Diagnostics | <10ms | Incremental |

**Memory Usage**:
- ~5MB base
- ~10MB per file (includes parsed AST + analysis)
- Shared strings and interning reduce overhead

## Design Decisions

### 1. Why Go-like Syntax?

**Pro**:
- Familiar to many developers
- Simple and readable
- Less syntax to learn
- Faster onboarding

**Con**:
- Different from Rust (learning curve for transpiled code)

**Decision**: Worth it for simplicity and approachability

### 2. Why Transpile to Rust vs Custom Backend?

**Pro**:
- Leverage Rust's mature ecosystem
- Cargo for dependencies
- LLVM optimizations
- Rust's safety guarantees

**Con**:
- Debugging shows Rust code, not Windjammer
- Limited by Rust's capabilities

**Decision**: Transpiling provides immediate value and ecosystem access

### 3. Why Automatic Ownership Inference?

**Pro**:
- Eliminates the hardest part of Rust
- Faster development
- More accessible to beginners

**Con**:
- May infer suboptimally in some cases
- Hides some control from advanced users

**Decision**: Provide escape hatches (`&`, `&mut`) for when needed

### 4. Why Salsa for LSP?

**Pro**:
- Battle-tested (rust-analyzer)
- Excellent performance
- Natural fit for incremental compilation
- Strong type safety

**Con**:
- Additional dependency
- Learning curve for contributors

**Decision**: Performance and reliability are critical for LSP

## Extension Points

### Adding a New Feature

#### 1. New Syntax

1. Add token to `lexer.rs`
2. Add AST node to `parser.rs`
3. Parse the syntax in `parser.rs`
4. Generate Rust code in `codegen.rs`

#### 2. New Analysis

1. Add query to `database.rs`
2. Implement query function
3. Use in diagnostics or completion

#### 3. New LSP Feature

1. Add capability in `handlers.rs::initialize()`
2. Implement handler method
3. Update VSCode extension if needed

### Testing Strategy

```
Unit Tests:
  - Lexer: Token recognition
  - Parser: AST construction
  - Analyzer: Ownership inference
  - Codegen: Rust output

Integration Tests:
  - Full compile pipeline
  - Example programs

LSP Tests:
  - Query correctness
  - Incremental updates
  - Protocol compliance
```

## Future Enhancements

### Short Term
- [ ] Implement go-to-definition
- [ ] Add find-references
- [ ] Improve error messages
- [ ] Add more built-in decorators

### Medium Term
- [ ] Macro system
- [ ] Generic types
- [ ] Trait-like interfaces
- [ ] Better async/await integration

### Long Term
- [ ] Self-hosting (Windjammer compiler in Windjammer)
- [ ] Direct LLVM backend (optional)
- [ ] Package manager integration
- [ ] Debugging support (source maps)

## Contributing

See areas where you can contribute:

1. **Parser**: Add support for more Rust syntax
2. **Analyzer**: Improve ownership inference heuristics
3. **LSP**: Implement missing features (rename, code actions)
4. **Examples**: More real-world examples
5. **Documentation**: Tutorials and guides
6. **Testing**: Increase test coverage

Read `CONTRIBUTING.md` for guidelines.

