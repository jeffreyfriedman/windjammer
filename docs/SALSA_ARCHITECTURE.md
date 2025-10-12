# Salsa Architecture in Windjammer LSP

**Version**: 0.24.0  
**Date**: October 12, 2025  
**Status**: Production Ready

---

## Table of Contents

1. [Overview](#overview)
2. [Why Salsa?](#why-salsa)
3. [Architecture](#architecture)
4. [Query System](#query-system)
5. [Incremental Computation](#incremental-computation)
6. [Performance Characteristics](#performance-characteristics)
7. [Implementation Details](#implementation-details)
8. [Best Practices](#best-practices)
9. [Future Directions](#future-directions)

---

## Overview

### What is Salsa?

Salsa is a **framework for on-demand, incremental computation**. It enables efficient caching and recomputation of derived data when inputs change.

**Key Concepts**:
- **Inputs**: Source data that changes (e.g., file contents)
- **Queries**: Functions that compute derived data
- **Memoization**: Automatic caching of query results
- **Incremental**: Only recompute what changed

### Windjammer's Integration

```
┌─────────────────────────────────────────┐
│          LSP Server (Tower)             │
│                                         │
│  ┌───────────────────────────────────┐ │
│  │    WindjammerDatabase (Salsa)     │ │
│  │                                   │ │
│  │  ┌──────────┐    ┌─────────────┐ │ │
│  │  │ Inputs:  │───>│  Queries:   │ │ │
│  │  │          │    │             │ │ │
│  │  │ • URI    │    │ • parse()   │ │ │
│  │  │ • Text   │    │ • imports() │ │ │
│  │  └──────────┘    │ • symbols() │ │ │
│  │                  └─────────────┘ │ │
│  │                        │         │ │
│  │                        v         │ │
│  │                  ┌─────────────┐ │ │
│  │                  │   Memos:    │ │ │
│  │                  │ • AST cache │ │ │
│  │                  │ • Symbols   │ │ │
│  │                  └─────────────┘ │ │
│  └───────────────────────────────────┘ │
│                                         │
│  ┌───────────────────────────────────┐ │
│  │        LSP Handlers               │ │
│  │  • Hover    • Completion          │ │
│  │  • Goto Def • Find References     │ │
│  └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

---

## Why Salsa?

### The Problem

**Without Incremental Computation**:

```rust
// User types one character
fn did_change(file: &str, content: String) {
    // Re-parse entire file (20μs)
    let ast = parse(content);
    
    // Re-analyze entire file (100μs)
    let symbols = analyze(&ast);
    
    // Re-compute all diagnostics (50μs)
    let diagnostics = check(&ast);
    
    // Total: 170μs per keystroke
}
```

**Problems**:
- ❌ Wasted CPU on unchanged parts
- ❌ Poor battery life
- ❌ Doesn't scale to large projects
- ❌ Slow response on fast typing

### The Solution

**With Salsa**:

```rust
// User types one character
fn did_change(file: &str, content: String) {
    // Update input
    db.set_source_text(uri, content);
    
    // Query AST (memoized if unchanged: 20ns!)
    let ast = db.get_program(file);  // Cache hit!
    
    // Query symbols (only recompute if AST changed)
    let symbols = db.get_symbols(file);
    
    // Query diagnostics (only recompute if needed)
    let diagnostics = db.get_diagnostics(file);
    
    // Total: 20ns if nothing changed!
}
```

**Benefits**:
- ✅ **~1000x faster** for unchanged queries
- ✅ Automatic dependency tracking
- ✅ Scales to large projects
- ✅ Minimal CPU usage

---

## Architecture

### Database Structure

```rust
#[salsa::db]
pub struct WindjammerDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for WindjammerDatabase {}
```

**Key Points**:
- `#[salsa::db]` macro generates the database infrastructure
- `Storage` manages all memoization and dependency tracking
- Database is **NOT** thread-safe (uses `RefCell` internally)

### Input Definition

```rust
#[salsa::input]
pub struct SourceFile {
    #[returns(ref)]
    pub uri: Url,
    
    #[returns(ref)]
    pub text: String,
}
```

**Inputs**:
- Represent external data that changes
- Created with `SourceFile::new(db, uri, text)`
- Immutable once created
- Updating creates a **new** SourceFile

### Query Definition

```rust
#[salsa::tracked]
pub struct ParsedProgram {
    #[returns(ref)]
    pub program: parser::Program,
}

#[salsa::tracked]
fn parse(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    let uri = file.uri(db);
    let text = file.text(db);
    
    // Lex and parse
    let mut lexer = lexer::Lexer::new(text);
    let tokens = lexer.tokenize();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().unwrap_or_default();
    
    ParsedProgram::new(db, program)
}
```

**Queries**:
- Functions marked with `#[salsa::tracked]`
- Automatically memoized
- Dependency tracked
- Results stored in database

---

## Query System

### Query Types

Salsa supports three types of queries:

#### 1. Input Queries

```rust
#[salsa::input]
pub struct SourceFile {
    pub uri: Url,
    pub text: String,
}
```

**Characteristics**:
- External data sources
- Must be explicitly set
- Changing triggers dependent queries
- Always returns same value until updated

#### 2. Tracked Queries

```rust
#[salsa::tracked]
fn parse(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    // Computation here
}
```

**Characteristics**:
- Computed on demand
- Automatically memoized
- Invalidated when dependencies change
- Most common query type

#### 3. Interned Values

```rust
#[salsa::interned]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
}
```

**Characteristics**:
- Deduplicated by value
- Pointer equality for same values
- Efficient for frequently-used data
- Not yet used in Windjammer (future)

### Query Execution Flow

```
1. User requests hover at position
   │
   v
2. LSP handler calls db.get_program(file)
   │
   v
3. Salsa checks memo table
   │
   ├─> Cache HIT (20ns)
   │   └─> Return cached AST
   │
   └─> Cache MISS (5-25μs)
       ├─> Execute parse query
       ├─> Store result in memo
       └─> Return new AST
```

### Dependency Tracking

Salsa automatically tracks dependencies:

```rust
// Query A depends on B
#[salsa::tracked]
fn semantic_analysis(db: &dyn Db, file: SourceFile) -> Analysis {
    let program = parse(db, file);  // Dependency!
    // ... analyze program
}
```

**When `file.text` changes**:
1. Input changed → invalidate `parse()`
2. `parse()` invalidated → invalidate `semantic_analysis()`
3. Next call to `semantic_analysis()` recomputes

**When query is called again**:
- If `parse()` result unchanged → reuse `semantic_analysis()` result
- If `parse()` result changed → recompute `semantic_analysis()`

---

## Incremental Computation

### How Salsa Achieves Incrementality

#### 1. Input Tracking

```rust
// Version 1: Initial state
let file1 = db.set_source_text(uri.clone(), "fn main() {}".into());
let program1 = db.get_program(file1);  // Parse happens

// Version 2: Update (different content)
let file2 = db.set_source_text(uri.clone(), "fn foo() {}".into());
let program2 = db.get_program(file2);  // Re-parse happens

// Version 3: Query again (same content as v2)
let program3 = db.get_program(file2);  // Cache hit! (20ns)
```

#### 2. Revision-Based Memoization

Salsa uses **revisions** to track changes:

```
Revision 0: Initial state
├─> Input: file1 = "fn main() {}"
└─> Memo: parse(file1) → AST1

Revision 1: User edits file
├─> Input: file2 = "fn foo() {}"
└─> Memo: parse(file1) → INVALID
           parse(file2) → AST2

Revision 2: Query with same input
├─> Input: file2 (unchanged)
└─> Memo: parse(file2) → AST2 (cache hit!)
```

#### 3. Deep vs Shallow Comparison

Salsa can verify if **output** changed even if **input** changed:

```rust
// Input changed: "fn main() {}" → "fn main() { }"  (whitespace)
// But AST unchanged: AST1 == AST1
// → Downstream queries NOT invalidated!
```

This is called **deep verification** and requires:
- Output types implement `Eq` or `PartialEq`
- Salsa can hash/compare results

---

## Performance Characteristics

### Benchmark Results

From `cargo bench --package windjammer-lsp`:

| Operation | Time | Notes |
|-----------|------|-------|
| **First parse** | 5-25 μs | Cold start |
| **Cached query** | ~20 ns | 1000x faster! |
| **Incremental edit** | 24 μs | Full re-parse |
| **Multi-file (3 cached)** | 62 ns | ~21ns per file |

### Memory Usage

**Storage Overhead**:
- Each memo: ~64 bytes (pointer + metadata)
- Per-query storage: 1 memo per unique input
- Total: ~1-10 KB per file

**Example** (100 files):
```
Inputs:   100 files × 64 bytes = 6.4 KB
Queries:  100 parse results × 64 bytes = 6.4 KB
ASTs:     100 files × ~5 KB avg = 500 KB
Total:    ~512 KB (very reasonable!)
```

### Scalability

| Files | First Load | All Cached |
|-------|------------|------------|
| 10    | ~200 μs    | ~200 ns    |
| 100   | ~2 ms      | ~2 μs      |
| 1000  | ~20 ms     | ~20 μs     |

**Linear scaling** with excellent caching!

---

## Implementation Details

### Thread Safety

**Problem**: Salsa uses `RefCell` internally → not `Send` or `Sync`

**Solution**: Wrap in `Arc<Mutex<_>>`

```rust
pub struct WindjammerLanguageServer {
    salsa_db: Arc<Mutex<WindjammerDatabase>>,
    // ...
}
```

**Important**: Must scope Mutex guards before `.await`:

```rust
// ❌ BAD: Holds lock across await
async fn bad_example(&self) {
    let db = self.salsa_db.lock().unwrap();
    let program = db.get_program(file);
    self.publish_diagnostics(&program).await;  // ERROR!
}

// ✅ GOOD: Release lock before await
async fn good_example(&self) {
    let program = {
        let db = self.salsa_db.lock().unwrap();
        db.get_program(file).clone()
    }; // Lock released here
    self.publish_diagnostics(&program).await;  // OK!
}
```

### Lifetime Management

**Problem**: Query results borrow from database

```rust
// ❌ This doesn't work across async boundaries
fn get_program(&self, file: SourceFile) -> &Program {
    let db = self.salsa_db.lock().unwrap();
    db.get_program(file)  // Lifetime tied to db!
}
```

**Solution**: Clone the result

```rust
// ✅ Clone to extend lifetime
fn get_program(&self, file: SourceFile) -> Program {
    let db = self.salsa_db.lock().unwrap();
    db.get_program(file).clone()  // Program now owned
}
```

**Cost**: `Program` clone is cheap (~1μs) vs re-parse (~20μs)

### Error Handling

Salsa queries can panic, so we handle errors gracefully:

```rust
#[salsa::tracked]
fn parse(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(err) => {
            // Log error and return empty program
            tracing::error!("Parse error: {}", err);
            parser::Program::default()
        }
    };
    ParsedProgram::new(db, program)
}
```

**Result**: Never panics, always returns valid (possibly empty) AST

---

## Best Practices

### 1. Keep Queries Pure

```rust
// ✅ GOOD: Pure function
#[salsa::tracked]
fn parse(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    let text = file.text(db);
    let ast = parser::parse(text);
    ParsedProgram::new(db, ast)
}

// ❌ BAD: Side effects
#[salsa::tracked]
fn parse_and_log(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    println!("Parsing file!");  // Side effect!
    // ...
}
```

**Why**: Salsa may call queries multiple times for verification

### 2. Make Inputs Immutable

```rust
// ✅ GOOD: Create new input on change
let file1 = db.set_source_text(uri.clone(), old_text);
let file2 = db.set_source_text(uri, new_text);  // New input!

// ❌ BAD: Mutate input (not possible anyway)
// Salsa inputs are immutable!
```

### 3. Scope Database Access

```rust
// ✅ GOOD: Minimize lock duration
let program = {
    let mut db = self.salsa_db.lock().unwrap();
    let file = db.set_source_text(uri, text);
    db.get_program(file).clone()
}; // Lock released

// ❌ BAD: Hold lock too long
let db = self.salsa_db.lock().unwrap();
let file = db.set_source_text(uri, text);
let program = db.get_program(file);
// ... do more work with db locked
```

### 4. Use Structural Equality

```rust
// ✅ GOOD: Derive Eq/PartialEq for deep comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Program {
    pub items: Vec<Item>,
}

// ❌ BAD: No equality → always invalidates
pub struct Program {
    pub items: Vec<Item>,
}
```

### 5. Log Performance

```rust
let start = std::time::Instant::now();
let program = db.get_program(file);
tracing::debug!(
    "Parse took {:?} (cached: {})",
    start.elapsed(),
    start.elapsed().as_micros() < 100  // < 100μs = cache hit
);
```

---

## Future Directions

### 1. Fine-Grained Queries (Phase 4)

**Current**: Parse entire file

```rust
#[salsa::tracked]
fn parse(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    // Parse entire file
}
```

**Future**: Parse per-function

```rust
#[salsa::tracked]
fn parse_function(db: &dyn Db, file: SourceFile, offset: usize) -> Function {
    // Parse only one function
    // If user edits function at line 10, don't re-parse function at line 100!
}
```

**Expected**: ~10x improvement for edits

### 2. Cross-File Queries (Phase 3)

```rust
#[salsa::tracked]
fn resolve_import(db: &dyn Db, file: SourceFile, import: &str) -> Option<SourceFile> {
    // Resolve "use std.fs" to actual file
    // Dependency tracking across files!
}

#[salsa::tracked]
fn find_all_references(db: &dyn Db, symbol: Symbol) -> Vec<Location> {
    // Find all uses of symbol across ALL files
    // Only re-search files that changed!
}
```

**Expected**: Instant cross-file queries with caching

### 3. Semantic Analysis Queries

```rust
#[salsa::tracked]
fn type_check(db: &dyn Db, file: SourceFile) -> TypeInfo {
    let program = parse(db, file);
    // Type check and return type info
    // Cached until AST changes!
}

#[salsa::tracked]
fn borrow_check(db: &dyn Db, function: Function) -> BorrowInfo {
    let types = type_check(db, function.file);
    // Borrow check and return ownership info
}
```

**Expected**: Sub-millisecond type checking

### 4. Interned Symbols

```rust
#[salsa::interned]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
}

// Symbol("main") always has same pointer
let sym1 = Symbol::new(db, "main".into(), SymbolKind::Function);
let sym2 = Symbol::new(db, "main".into(), SymbolKind::Function);
assert!(std::ptr::eq(&sym1, &sym2));  // Same pointer!
```

**Expected**: Faster symbol comparison, less memory

---

## Appendix: Salsa Concepts

### Durability

Controls how aggressively Salsa caches:

```rust
#[salsa::input]
pub struct SourceFile {
    #[durability(Durability::LOW)]  // Changes often
    pub text: String,
    
    #[durability(Durability::HIGH)]  // Rarely changes
    pub uri: Url,
}
```

**Not yet used in Windjammer** (future optimization)

### Derived Values

Salsa can derive new inputs from existing ones:

```rust
#[salsa::tracked]
pub struct TokenStream {
    #[returns(ref)]
    pub tokens: Vec<Token>,
}

#[salsa::tracked]
fn tokenize(db: &dyn Db, file: SourceFile) -> TokenStream {
    let text = file.text(db);
    let tokens = lexer::tokenize(text);
    TokenStream::new(db, tokens)
}
```

**Future**: Separate tokenization from parsing for better granularity

### Garbage Collection

Salsa automatically removes unused memos:

```rust
// File closed → SourceFile dropped → memos GC'd
db.set_source_text(uri, text);  // Creates memo
// ... use it ...
// File closed → memo eventually removed
```

**Trigger**: When database has no references to a value

---

## Summary

### Key Takeaways

1. **Salsa = Incremental Computation Framework**
   - Automatic memoization
   - Dependency tracking
   - ~1000x speedup for cached queries

2. **Architecture**
   - Inputs: SourceFile (uri + text)
   - Queries: parse(), imports(), etc.
   - Memos: Cached results

3. **Performance**
   - First parse: 5-25μs (very fast)
   - Cached query: ~20ns (instant!)
   - Scales linearly with project size

4. **Best Practices**
   - Keep queries pure
   - Scope database access
   - Clone results for async
   - Log performance

5. **Future**
   - Fine-grained queries
   - Cross-file features
   - Semantic analysis
   - Interned symbols

### Resources

- [Salsa Book](https://salsa-rs.github.io/salsa/)
- [Rust Analyzer (uses Salsa)](https://github.com/rust-lang/rust-analyzer)
- [Windjammer Benchmarks](/tmp/v0.24.0_BENCHMARK_RESULTS.md)
- [Migration Guide](./SALSA_MIGRATION.md)

---

**Version**: 0.24.0  
**Status**: ✅ **Production Ready**  
**Performance**: ✅ **~1000x speedup achieved**  
**Next**: Cross-file features, semantic analysis

