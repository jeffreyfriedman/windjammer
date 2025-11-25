# Windjammer Standard Library

## Philosophy

The Windjammer standard library provides a "batteries included" experience by wrapping best-in-class Rust crates with clean, idiomatic Windjammer APIs.

### Design Principles

1. **Abstraction over Implementation**: Users import `std.json`, not `serde_json`
2. **Best-in-Class Wrapping**: We use the best Rust crates, not reinvent the wheel
3. **100% Rust Compatibility**: Can still use Rust crates directly when needed
4. **Consistent APIs**: All modules follow similar patterns
5. **Zero Configuration**: Auto-included in projects

---

## Structure

```
std/
  ├── json.wj       - JSON parsing (wraps serde_json)
  ├── http.wj       - HTTP client/server (wraps reqwest/hyper)
  ├── fs.wj         - File system (wraps std::fs/tokio::fs)
  ├── path.wj       - Path manipulation
  ├── io.wj         - I/O operations
  ├── time.wj       - Date/time (wraps chrono)
  ├── crypto.wj     - Cryptography (wraps ring)
  ├── encoding.wj   - Base64, hex, etc.
  ├── net.wj        - Networking
  ├── sync.wj       - Concurrency primitives
  ├── testing.wj    - Test framework
  ├── fmt.wj        - Formatting
  ├── strings.wj    - String utilities
  ├── math.wj       - Math functions
  ├── collections.wj - Data structures
  ├── regex.wj      - Regular expressions
  ├── cli.wj        - Command-line parsing (wraps clap)
  └── log.wj        - Logging (wraps tracing)
```

---

## Usage

### Explicit Import (Recommended)
```windjammer
use std::json

fn main() {
    let data = json::parse("{\"name\": \"Alice\"}")
    println!("{:?}", data)
}
```

### Future: Auto-Import (v0.5.0+)
```windjammer
// No import needed for common stdlib modules
fn main() {
    let data = json::parse("{\"name\": \"Alice\"}")
}
```

---

## Implementation

Each `std/*.wj` module:
1. Is a thin wrapper around a Rust crate
2. Provides a clean, Windjammer-idiomatic API
3. Re-exports types for advanced use
4. Handles common cases simply

### Example: std/json.wj

```windjammer
// User-facing API
fn parse(json_str: string) -> Result<Value, Error> {
    serde_json::from_str(json_str)  // Wraps Rust crate
}

fn stringify(value: &Value) -> Result<string, Error> {
    serde_json::to_string(value)
}

// Re-export for advanced users
type Value = serde_json::Value
type Error = serde_json::Error
```

**Transpiles to pure Rust**:
```rust
pub fn parse(json_str: String) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(&json_str)
}

pub fn stringify(value: &serde_json::Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}
```

---

## Dependency Management

### Auto-Inject Dependencies

When a user imports a stdlib module, the compiler automatically:
1. Detects which crates are needed
2. Adds them to `Cargo.toml` (if building new project)
3. Generates appropriate `use` statements

Example:
```windjammer
use std::json  // Compiler adds serde_json to dependencies
use std::http  // Compiler adds reqwest to dependencies
```

### Manual Override

Users can still add dependencies manually if they want specific versions:
```toml
[dependencies]
serde_json = "1.0.100"  # Override version
reqwest = { version = "0.11", features = ["json"] }
```

---

## Current Status

### ✅ Implemented (v0.4.0)
- `json.wj` - JSON parsing and serialization
- `http.wj` - HTTP client (server coming in v0.5.0)
- `fs.wj` - File system operations

### ✅ Implemented (v0.5.0) - Module System Complete!
- `time.wj` - Date/time operations (wraps chrono)
- `strings.wj` - String manipulation utilities
- `math.wj` - Mathematical functions and constants
- `log.wj` - Logging (wraps log/env_logger)
- `csv.wj` - CSV parsing and writing
- `regex.wj` - Regular expressions (wraps regex crate)
- `encoding.wj` - Base64, hex, URL encoding
- `crypto.wj` - Hashing functions (SHA256, MD5)

**✅ Module System**: Real Windjammer code, transpiled to Rust
**✅ Tested**: std/fs module working end-to-end!

### ✅ Implemented (v0.9.0) - Collections & Testing!
- `collections.wj` - HashMap, HashSet, BTreeMap, BTreeSet, VecDeque
- `testing.wj` - Testing framework with assertions

### 📋 Planned (v0.10.0)
- `cli.wj` - Command-line parsing (wraps clap)
- HTTP server support in `http.wj`
- `path.wj` - Path manipulation
- `sync.wj` - Concurrency primitives

---

## Testing

Each stdlib module includes:
1. Unit tests in Windjammer
2. Integration tests with real usage
3. Doc examples (Rust-style doctests)

Example:
```windjammer
// std/json.wj
@test
fn test_parse() {
    let result = parse("{\"name\": \"Alice\"}")
    assert!(result.is_ok())
}
```

---

## Future Enhancements

### v0.5.0: HTTP and FS Modules
- Complete `http.wj` with client and server support
- Complete `fs.wj` with async file operations

### v0.6.0: More Modules
- `time.wj`, `crypto.wj`, `regex.wj`
- Auto-import for common modules

### v0.10.0: Complete Standard Library
- All planned modules implemented
- Comprehensive documentation
- Performance benchmarks
- Production confidence-building period before v1.0.0

---

## Contributing

When adding a new stdlib module:

1. Choose the best-in-class Rust crate to wrap
2. Create `std/your_module.wj`
3. Provide simple, idiomatic API
4. Re-export types for advanced use
5. Add tests
6. Update this README

---

**Version**: v0.4.0  
**Status**: Foundation laid, initial modules in progress
