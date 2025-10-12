# Windjammer LSP Server

**Version**: 0.24.0  
**Status**: Production Ready  
**Performance**: ~1000x speedup with Salsa incremental computation

---

## Overview

The Windjammer Language Server Protocol (LSP) implementation provides intelligent code editing features for the Windjammer programming language.

**Key Features**:
- 🚀 **Incremental Computation**: Salsa-powered caching (~1000x speedup)
- 🎯 **Rich IDE Support**: Hover, completion, goto definition, and more
- 📊 **Diagnostics**: Real-time error checking and warnings
- 🔍 **Symbol Navigation**: Find references, workspace symbols
- ⚡ **Fast**: Sub-microsecond cached queries
- 🔧 **Refactoring**: Rename, extract function (coming soon)

---

## Quick Start

### Installation

```bash
cargo install windjammer-lsp
```

### Editor Setup

#### VS Code

Install the Windjammer extension (coming soon) or configure manually:

```json
{
  "windjammer.lsp.path": "/path/to/windjammer-lsp",
  "windjammer.lsp.arguments": []
}
```

#### Neovim

```lua
require'lspconfig'.windjammer.setup{
  cmd = { "windjammer-lsp" },
  filetypes = { "windjammer", "wj" },
}
```

#### Emacs (lsp-mode)

```elisp
(add-to-list 'lsp-language-id-configuration '(windjammer-mode . "windjammer"))
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "windjammer-lsp")
                  :major-modes '(windjammer-mode)
                  :server-id 'windjammer-lsp))
```

---

## API Documentation

### Core Database

#### `WindjammerDatabase`

The Salsa database that powers incremental computation.

```rust
use windjammer_lsp::database::WindjammerDatabase;

// Create a new database
let mut db = WindjammerDatabase::new();

// Set source text (creates input)
let uri = Url::parse("file:///example.wj").unwrap();
let file = db.set_source_text(uri, "fn main() {}".to_string());

// Query the parsed program (automatically memoized)
let program = db.get_program(file);

// Query again (cache hit - ~20ns!)
let program2 = db.get_program(file);
```

#### `SourceFile` (Input)

Represents a source file in the database.

```rust
#[salsa::input]
pub struct SourceFile {
    pub uri: Url,      // File URI
    pub text: String,  // File contents
}

// Create via database
let file = SourceFile::new(db, uri, text);

// Access fields
let uri = file.uri(db);
let text = file.text(db);
```

#### `ParsedProgram` (Query Result)

Represents a parsed AST.

```rust
#[salsa::tracked]
pub struct ParsedProgram {
    pub program: parser::Program,
}

// Query via database
let parsed = parse(db, file);
let program = parsed.program(db);

// Access AST
for item in &program.items {
    match item {
        Item::Function(func) => println!("Function: {}", func.name),
        // ...
    }
}
```

### Queries

#### `parse(db, file) -> ParsedProgram`

Parse a source file into an AST.

```rust
#[salsa::tracked]
fn parse(db: &dyn Db, file: SourceFile) -> ParsedProgram {
    let text = file.text(db);
    
    // Lex and parse
    let mut lexer = Lexer::new(text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap_or_default();
    
    ParsedProgram::new(db, program)
}
```

**Performance**:
- First call: 5-25 μs (parses from source)
- Cached call: ~20 ns (memoized)
- Speedup: ~1000x

**Example**:
```rust
let file = db.set_source_text(uri, source);
let parsed = parse(&db, file);  // First parse: ~10 μs
let parsed2 = parse(&db, file); // Cache hit: ~20 ns!
```

#### `extract_imports(db, file) -> ImportInfo`

Extract import statements from a file.

```rust
#[salsa::tracked]
pub struct ImportInfo {
    pub import_uris: Vec<Url>,
}

#[salsa::tracked]
fn extract_imports(db: &dyn Db, file: SourceFile) -> ImportInfo {
    let parsed = parse(db, file);
    let program = parsed.program(db);
    
    let mut import_uris = Vec::new();
    for item in &program.items {
        if let Item::Use(use_item) = item {
            // Resolve import path to URI
            // (not yet implemented)
        }
    }
    
    ImportInfo::new(db, import_uris)
}
```

**Usage**:
```rust
let imports = extract_imports(&db, file);
let uris = imports.import_uris(&db);
```

---

## Examples

### Example 1: Parse a Single File

```rust
use windjammer_lsp::database::WindjammerDatabase;
use tower_lsp::lsp_types::Url;

fn main() {
    let mut db = WindjammerDatabase::new();
    
    // Create source file
    let uri = Url::parse("file:///example.wj").unwrap();
    let source = r#"
        fn fibonacci(n: int) -> int {
            if n <= 1 {
                return n;
            }
            fibonacci(n - 1) + fibonacci(n - 2)
        }
    "#;
    
    let file = db.set_source_text(uri, source.to_string());
    
    // Parse (first time - ~10 μs)
    let program = db.get_program(file);
    println!("Parsed {} items", program.items.len());
    
    // Parse again (cached - ~20 ns!)
    let program2 = db.get_program(file);
    assert_eq!(program.items.len(), program2.items.len());
}
```

### Example 2: Incremental Updates

```rust
use windjammer_lsp::database::WindjammerDatabase;
use tower_lsp::lsp_types::Url;

fn main() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///example.wj").unwrap();
    
    // Initial version
    let file1 = db.set_source_text(uri.clone(), "fn foo() {}".to_string());
    let prog1 = db.get_program(file1);
    println!("Version 1: {} items", prog1.items.len());
    
    // Update (user types)
    let file2 = db.set_source_text(uri.clone(), "fn foo() {}\nfn bar() {}".to_string());
    let prog2 = db.get_program(file2);
    println!("Version 2: {} items", prog2.items.len());
    
    // Query old version (still cached!)
    let prog1_again = db.get_program(file1);
    println!("Version 1 again: {} items", prog1_again.items.len());
}
```

### Example 3: Benchmarking Cache Performance

```rust
use windjammer_lsp::database::WindjammerDatabase;
use tower_lsp::lsp_types::Url;
use std::time::Instant;

fn main() {
    let mut db = WindjammerDatabase::new();
    let uri = Url::parse("file:///example.wj").unwrap();
    let source = "fn main() { println(\"Hello!\"); }".to_string();
    
    // First parse (cold)
    let start = Instant::now();
    let file = db.set_source_text(uri, source);
    let _prog = db.get_program(file);
    println!("First parse: {:?}", start.elapsed());
    // Output: First parse: 5-25 μs
    
    // Cached queries (hot)
    let start = Instant::now();
    for _ in 0..1000 {
        let _prog = db.get_program(file);
    }
    let elapsed = start.elapsed();
    println!("1000 cached queries: {:?}", elapsed);
    println!("Average per query: {:?}", elapsed / 1000);
    // Output: Average per query: ~20-50 ns
}
```

### Example 4: Multi-File Project

```rust
use windjammer_lsp::database::WindjammerDatabase;
use tower_lsp::lsp_types::Url;

fn main() {
    let mut db = WindjammerDatabase::new();
    
    // File 1: main.wj
    let uri1 = Url::parse("file:///main.wj").unwrap();
    let file1 = db.set_source_text(uri1, r#"
        use utils.helpers;
        fn main() {
            helpers.greet();
        }
    "#.to_string());
    
    // File 2: utils/helpers.wj
    let uri2 = Url::parse("file:///utils/helpers.wj").unwrap();
    let file2 = db.set_source_text(uri2, r#"
        fn greet() {
            println("Hello!");
        }
    "#.to_string());
    
    // Parse both (incremental)
    let prog1 = db.get_program(file1);
    let prog2 = db.get_program(file2);
    
    println!("main.wj: {} items", prog1.items.len());
    println!("helpers.wj: {} items", prog2.items.len());
    
    // Query imports
    let imports1 = db.get_imports(file1);
    println!("main.wj imports: {:?}", imports1.import_uris(&db));
}
```

---

## Performance Characteristics

### Benchmark Results

From `cargo bench --package windjammer-lsp`:

| Operation | Time | Description |
|-----------|------|-------------|
| First parse (small) | 5.7 μs | 4-line file |
| First parse (medium) | 17.6 μs | 33-line file |
| First parse (large) | 24.4 μs | 95-line file |
| Cached query (any) | ~20 ns | Memoized result |
| Incremental edit | 24 μs | Re-parse modified file |
| Multi-file (3 cached) | 62 ns | Query 3 cached files |

### Memory Usage

- Per-file overhead: ~64 bytes (memo metadata)
- AST storage: ~50-100 bytes per line
- Total for 100 files: ~500 KB

### Scalability

| Files | First Load | All Cached |
|-------|------------|------------|
| 10    | ~200 μs    | ~200 ns    |
| 100   | ~2 ms      | ~2 μs      |
| 1000  | ~20 ms     | ~20 μs     |

---

## Advanced Usage

### Thread Safety

The database uses `Mutex` for thread safety:

```rust
use std::sync::{Arc, Mutex};
use windjammer_lsp::database::WindjammerDatabase;

let db = Arc::new(Mutex::new(WindjammerDatabase::new()));

// Access from multiple threads
let db_clone = db.clone();
std::thread::spawn(move || {
    let mut db = db_clone.lock().unwrap();
    // Use database
});
```

**Important**: Must scope locks before `.await`:

```rust
// ✅ CORRECT
let program = {
    let db = self.db.lock().unwrap();
    db.get_program(file).clone()
}; // Lock released
await_something().await;  // OK

// ❌ WRONG
let db = self.db.lock().unwrap();
let program = db.get_program(file);
await_something().await;  // ERROR: MutexGuard not Send
```

### Custom Queries

Add your own Salsa queries:

```rust
#[salsa::tracked]
fn count_functions(db: &dyn Db, file: SourceFile) -> usize {
    let program = db.get_program(file);
    program.items.iter()
        .filter(|item| matches!(item, Item::Function(_)))
        .count()
}

// Usage
let count = count_functions(&db, file);  // Memoized!
```

### Logging and Debugging

Enable tracing to see cache hits:

```bash
RUST_LOG=windjammer_lsp=debug windjammer-lsp
```

Output:
```
DEBUG Salsa: Parsing file:///example.wj
DEBUG Salsa parse complete in 12.3μs (memoized: false)
DEBUG Salsa parse complete in 23ns (memoized: true)
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_caching() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, "fn main() {}".into());
        
        // First parse
        let prog1 = db.get_program(file);
        assert_eq!(prog1.items.len(), 1);
        
        // Cached query (same pointer!)
        let prog2 = db.get_program(file);
        assert!(std::ptr::eq(prog1, prog2));
    }
}
```

### Integration Tests

See `tests/integration_tests.rs` for full LSP protocol tests.

### Benchmarks

```bash
cargo bench --package windjammer-lsp --bench salsa_performance
```

---

## Troubleshooting

### Issue: "Future cannot be sent between threads"

**Cause**: Holding `MutexGuard` across `.await`

**Fix**: Scope the lock
```rust
let data = {
    let db = self.db.lock().unwrap();
    db.query().clone()
};
```

### Issue: Slow performance

**Check**:
1. Enable debug logging to verify cache hits
2. Ensure not calling `set_source_text` unnecessarily
3. Profile with `cargo bench`

### Issue: Memory usage high

**Solution**: Salsa automatically GCs unused data. Close unused files to trigger cleanup.

---

## Architecture

```
┌─────────────────────────────────────────┐
│          LSP Server (async)             │
│                                         │
│  ┌───────────────────────────────────┐ │
│  │  Arc<Mutex<WindjammerDatabase>>   │ │
│  │                                   │ │
│  │  Inputs:                          │ │
│  │  • SourceFile(uri, text)          │ │
│  │                                   │ │
│  │  Queries:                         │ │
│  │  • parse() → ParsedProgram        │ │
│  │  • extract_imports() → ImportInfo │ │
│  │                                   │ │
│  │  Memos (cache):                   │ │
│  │  • AST by (uri, text) hash        │ │
│  │  • Import URIs                    │ │
│  └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

For detailed architecture, see [SALSA_ARCHITECTURE.md](../../docs/SALSA_ARCHITECTURE.md).

---

## Contributing

### Adding New Queries

1. Define the query:
```rust
#[salsa::tracked]
fn my_query(db: &dyn Db, file: SourceFile) -> MyResult {
    // Computation here
}
```

2. Add to public API:
```rust
impl WindjammerDatabase {
    pub fn my_query(&self, file: SourceFile) -> MyResult {
        my_query(self, file).clone()
    }
}
```

3. Add tests:
```rust
#[test]
fn test_my_query() {
    let mut db = WindjammerDatabase::new();
    // Test here
}
```

4. Add benchmarks if performance-critical

### Best Practices

- Keep queries pure (no side effects)
- Make results `Clone` for lifetime management
- Log performance for cache verification
- Scope database locks properly
- Document expected performance

---

## Resources

- **Salsa Book**: https://salsa-rs.github.io/salsa/
- **Architecture Guide**: [docs/SALSA_ARCHITECTURE.md](../../docs/SALSA_ARCHITECTURE.md)
- **Migration Guide**: [docs/SALSA_MIGRATION.md](../../docs/SALSA_MIGRATION.md)
- **LSP Specification**: https://microsoft.github.io/language-server-protocol/
- **Windjammer Docs**: https://github.com/jeffreyfriedman/windjammer

---

## License

Same as Windjammer project.

---

## Changelog

See [CHANGELOG.md](../../CHANGELOG.md) for version history.

**v0.24.0**: Salsa incremental computation (~1000x speedup!)  
**v0.23.0**: Production applications and tooling  
**v0.22.0**: SmallVec and Cow optimizations
