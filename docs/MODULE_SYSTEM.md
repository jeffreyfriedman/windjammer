# Module System

## Overview

Windjammer features a comprehensive module system that allows you to organize code and use the standard library. The key innovation is that **stdlib modules are real Windjammer code** that gets transpiled to Rust, making them transparent, readable, and community-friendly.

## Importing Modules

Use the `use` keyword to import modules:

```windjammer
use std::fs
use std::json

fn main() {
    // Use imported modules
    let data = fs::read_to_string("file.txt")
    println!("{:?}", data)
}
```

## Standard Library Modules

Windjammer provides a "batteries included" standard library covering common use cases:

### Core I/O
- **std.fs** - File system operations (read, write, exists, etc.)
- **std.json** - JSON parsing and serialization
- **std.csv** - CSV data processing
- **std.http** - HTTP client for API requests

### Utilities
- **std.time** - Date/time handling and formatting
- **std.strings** - String manipulation functions
- **std.math** - Mathematical functions and constants
- **std.log** - Logging framework

### Data Processing
- **std.regex** - Regular expression matching
- **std.encoding** - Base64, hex, URL encoding/decoding
- **std.crypto** - Cryptographic hashing (SHA256, MD5)

## Module System Design

### Architecture: Transpiled Modules

Unlike many languages where the standard library is implemented in a different language (C, assembly, etc.), Windjammer's stdlib is **written entirely in Windjammer**. This provides several benefits:

1. **Transparency** - You can read the source code to understand how it works
2. **Learning Resource** - Stdlib serves as canonical examples of good Windjammer code
3. **Community-Friendly** - Easy to contribute improvements via pull requests
4. **Dogfooding** - Proves Windjammer can write real, practical code

### How It Works

When you write:
```windjammer
use std::fs

fn main() {
    fs.exists("/tmp")
}
```

The compiler:
1. Finds `std/fs.wj` in the stdlib directory
2. Transpiles it to Rust code
3. Wraps it in a `pub mod fs { ... }` block
4. Converts `fs.exists()` to `fs::exists()`
5. Includes it in your output

Generated Rust:
```rust
pub mod fs {
    pub fn exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }
    // ... more functions
}

fn main() {
    fs::exists("/tmp");
}
```

## Using the File System Module

### Reading Files

```windjammer
use std::fs

fn main() {
    // Read entire file as string
    match fs::read_to_string("config.txt") {
        Ok(content) => println!("File content: {}", content),
        Err(e) => println!("Error: {}", e),
    }
    
    // Read as bytes
    match fs::read("data.bin") {
        Ok(bytes) => println!("Read {} bytes", bytes.len()),
        Err(e) => println!("Error: {}", e),
    }
}
```

### Writing Files

```windjammer
use std::fs

fn main() {
    // Write string to file
    match fs::write("output.txt", "Hello, World!") {
        Ok(_) => println!("File written successfully"),
        Err(e) => println!("Error: {}", e),
    }
    
    // Write bytes
    let data: Vec<u8> = vec![0, 1, 2, 3]
    fs::write_bytes("output.bin", &data)
}
```

### File System Queries

```windjammer
use std::fs

fn main() {
    // Check if path exists
    if fs.exists("/tmp") {
        println!("Path exists!")
    }
    
    // Check if it's a file or directory
    if fs.is_file("data.txt") {
        println!("It's a file")
    }
    
    if fs.is_dir("/tmp") {
        println!("It's a directory")
    }
}
```

### Directory Operations

```windjammer
use std::fs

fn main() {
    // Create directory (including parents)
    fs.create_dir_all("/tmp/my_app/data")
    
    // Remove file
    fs.remove_file("old_file.txt")
    
    // Remove directory
    fs.remove_dir("/tmp/empty_dir")
    
    // Remove directory recursively
    fs.remove_dir_all("/tmp/full_dir")
    
    // Copy file
    fs.copy("source.txt", "dest.txt")
    
    // Rename/move file
    fs.rename("old_name.txt", "new_name.txt")
}
```

## Using the JSON Module

```windjammer
use std::json

fn main() {
    // Parse JSON string
    let json_str = "{\"name\": \"Alice\", \"age\": 30}"
    
    match json::parse(json_str) {
        Ok(value) => {
            // Check type
            if json.is_object(&value) {
                println!("It's an object!")
            }
            
            // Get field
            if let Some(name) = json.get(&value, "name") {
                if let Some(name_str) = json.as_str(name) {
                    println!("Name: {}", name_str)
                }
            }
            
            // Get numeric field
            if let Some(age) = json.get(&value, "age") {
                if let Some(age_num) = json.as_i64(age) {
                    println!("Age: {}", age_num)
                }
            }
        },
        Err(e) => println!("Parse error: {}", e),
    }
}
```

## Using Multiple Modules

You can import and use multiple modules together:

```windjammer
use std::fs
use std::json

fn load_config(path: &str) -> Option<serde_json.Value> {
    match fs::read_to_string(path) {
        Ok(content) => {
            match json::parse(&content) {
                Ok(value) => Some(value),
                Err(e) => {
                    println!("JSON parse error: {}", e)
                    None
                }
            }
        },
        Err(e) => {
            println!("File read error: {}", e)
            None
        }
    }
}

fn main() {
    if let Some(config) = load_config("config.json") {
        println!("Loaded config: {:?}", config)
    }
}
```

## Module Resolution

### Standard Library Path

The compiler looks for stdlib modules in the `std/` directory. You can override this with the `WINDJAMMER_STDLIB` environment variable:

```bash
export WINDJAMMER_STDLIB=/path/to/custom/stdlib
```

### Import Syntax

- **Standard library**: `use std::module_name`
- **Submodules** (future): `use std::http.client`
- **Qualified imports** (future): `use std::fs as filesystem`

Currently, only `std.*` imports are supported. User modules and relative imports will be added in future versions.

## Best Practices

### 1. Import What You Need
```windjammer
// Good: Only import what you use
use std::fs

fn main() {
    fs::read_to_string("file.txt")
}

// Avoid: Importing unused modules
use std::json  // Not used
use std::http  // Not used
```

### 2. Handle Errors Properly
```windjammer
use std::fs

// Good: Handle errors explicitly
fn read_config() -> Result<String, std::io::Error> {
    fs::read_to_string("config.txt")
}

// Better: Provide context
fn read_config_safe() -> String {
    match fs::read_to_string("config.txt") {
        Ok(content) => content,
        Err(_) => {
            println!("Using default config")
            "default_config".to_string()
        }
    }
}
```

### 3. Keep Module Usage Clean
```windjammer
use std::fs
use std::json

// Good: Clear module prefixes
fn process_data() {
    let data = fs::read_to_string("data.json").unwrap()
    let parsed = json::parse(&data).unwrap()
    // ...
}
```

## Future Enhancements

The module system will be expanded in future versions:

### v0.6.0
- User-defined modules (not just stdlib)
- Relative imports (`use ./my_module`)
- Module aliases (`use std::fs as filesystem`)
- Re-exports (`pub use`)

### v0.7.0
- Submodule imports (`use std::http.client`)
- Selective imports (`use std::fs.{read, write}`)
- Private vs public module items
- Cross-module visibility control

### v0.10.0
- Module caching for faster compilation
- Precompiled stdlib binaries
- Module documentation generation
- Workspace/package system
- Production confidence-building before v1.0.0

## Contributing to Stdlib

Since stdlib modules are written in Windjammer, contributing is straightforward:

1. **Read the source**: Check `std/*.wj` files to understand patterns
2. **Follow conventions**: Match existing module structure
3. **Add functions**: Write clear, documented functions
4. **Test it**: Create test examples
5. **Submit PR**: Contribute back to the community

Example stdlib module structure:
```windjammer
// std/mymodule.wj - Brief description

// Public function with clear purpose
fn do_something(input: &str) -> Result<String, Error> {
    // Implementation that calls Rust crates
    my_crate::function(input)
}

// Another public function
fn do_something_else(data: &[u8]) -> Vec<u8> {
    // ...
}
```

## See Also

- [Standard Library Reference](../std/README.md) - Complete API documentation
- [Examples](../examples/) - Working code examples
- [Rust Interoperability](COMPARISON.md#rust-interoperability) - Calling Rust from Windjammer

---

**The module system makes Windjammer's "batteries included" philosophy a reality, providing everything you need for common tasks without external dependencies.**
