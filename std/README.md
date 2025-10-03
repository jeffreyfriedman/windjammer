# Windjammer Standard Library

## Philosophy

**"Batteries Included"** - The Windjammer standard library aims to cover 80% of developer use cases without external dependencies.

### Design Principles

1. **Wrap Best-in-Class Rust Crates** - No need to reinvent the wheel
2. **Consistent API** - Uniform patterns across all modules
3. **Auto-included** - No manual Cargo.toml editing needed
4. **Windjammer Ergonomics** - Leverage string interpolation, pipe operator, etc.
5. **Zero Overhead** - Thin wrappers that compile away

### Usage

```windjammer
use std.fs
use std.http
use std.json

fn main() {
    // File operations
    let content = fs.read_to_string("data.txt")?
    
    // HTTP requests
    let response = http.get("https://api.example.com/users")?
    
    // JSON parsing
    let users = json.parse(response.body)?
    
    println!("Loaded ${users.len()} users")
}
```

## Modules

### Core I/O
- **`std.fs`** - File system operations (wraps `std::fs`)
- **`std.path`** - Path manipulation (wraps `std::path`)
- **`std.io`** - Input/output traits and utilities (wraps `std::io`)

### Networking
- **`std.http`** - HTTP client (wraps `reqwest`)
- **`std.net`** - TCP/UDP networking (wraps `std::net`)

### Data Formats
- **`std.json`** - JSON serialization (wraps `serde_json`)
- **`std.toml`** - TOML parsing (wraps `toml`)
- **`std.yaml`** - YAML parsing (wraps `serde_yaml`)

### Collections
- **`std.collections`** - HashMap, HashSet, etc. (wraps `std::collections`)

### Utilities
- **`std.fmt`** - Formatting and string manipulation
- **`std.testing`** - Test framework and assertions
- **`std.time`** - Time and duration (wraps `std::time`)
- **`std.os`** - OS-specific functionality (wraps `std::env`, `std::process`)

### Concurrency
- **`std.sync`** - Synchronization primitives (wraps `std::sync`)
- **`std.channels`** - Message passing (wraps `std::sync::mpsc`)

### Crypto & Encoding
- **`std.crypto`** - Cryptographic functions (wraps `sha2`, `blake3`)
- **`std.encoding`** - Base64, hex, etc. (wraps `base64`, `hex`)

### Development Tools
- **`std.cli`** - CLI argument parsing (wraps `clap`)
- **`std.log`** - Logging (wraps `tracing`)
- **`std.regex`** - Regular expressions (wraps `regex`)

## Implementation Strategy

### Phase 1: Core Essentials (Current)
- ✅ Directory structure
- 🔄 `std.testing` - Basic test framework
- 🔄 `std.fs` - File operations
- 🔄 `std.json` - JSON handling

### Phase 2: Network & Data
- `std.http` - HTTP client
- `std.collections` - Advanced data structures

### Phase 3: Complete Coverage
- All remaining modules
- Full documentation
- Comprehensive examples

## Auto-inclusion Mechanism

The Windjammer compiler automatically:
1. Detects `use std.*` imports
2. Adds corresponding Rust crate dependencies to `Cargo.toml`
3. Generates appropriate `use` statements in Rust output

Example:
```windjammer
use std.http
use std.json
```

Automatically generates in `Cargo.toml`:
```toml
[dependencies]
reqwest = { version = "0.11", features = ["blocking", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Benefits

✅ **No Dependency Hell** - Curated, compatible versions  
✅ **Instant Productivity** - Common tasks work out of the box  
✅ **Consistent Experience** - Uniform APIs across domains  
✅ **Best Practices** - Leverages proven Rust ecosystem  
✅ **Zero Overhead** - Thin wrappers, zero runtime cost  

---

*Status: In Development*  
*Version: 0.1.0*  
*Last Updated: October 2, 2025*

