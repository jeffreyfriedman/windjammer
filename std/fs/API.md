# std.fs - File System Operations

Thin wrapper around Rust's `std::fs` with Windjammer ergonomics.

## API

### Reading Files

```windjammer
use std.fs

// Read entire file as string
fn read_to_string(path: string) -> Result<string, Error>

// Read file as bytes
fn read(path: string) -> Result<Vec<u8>, Error>

// Read file line by line (iterator)
fn read_lines(path: string) -> Result<Vec<string>, Error>
```

### Writing Files

```windjammer
// Write string to file
fn write(path: string, contents: string) -> Result<(), Error>

// Write bytes to file
fn write_bytes(path: string, contents: Vec<u8>) -> Result<(), Error>

// Append to file
fn append(path: string, contents: string) -> Result<(), Error>
```

### File Operations

```windjammer
// Copy file
fn copy(from: string, to: string) -> Result<(), Error>

// Move/rename file
fn rename(from: string, to: string) -> Result<(), Error>

// Delete file
fn remove_file(path: string) -> Result<(), Error>

// Check if file exists
fn exists(path: string) -> bool

// Get file metadata
fn metadata(path: string) -> Result<Metadata, Error>
```

### Directory Operations

```windjammer
// Create directory
fn create_dir(path: string) -> Result<(), Error>

// Create directory and parents
fn create_dir_all(path: string) -> Result<(), Error>

// Remove directory
fn remove_dir(path: string) -> Result<(), Error>

// Remove directory and contents
fn remove_dir_all(path: string) -> Result<(), Error>

// List directory contents
fn read_dir(path: string) -> Result<Vec<DirEntry>, Error>
```

## Example Usage

```windjammer
use std.fs

fn backup_config() -> Result<(), Error> {
    // Read
    let config = fs.read_to_string("config.toml")?
    
    // Process with pipe operator
    let processed = config
        |> String.trim
        |> add_timestamp
    
    // Write
    fs.write("config.backup.toml", processed)?
    
    println!("Backup created successfully")
    Ok(())
}

fn process_logs() -> Result<(), Error> {
    // Read all log files
    let entries = fs.read_dir("logs")?
    
    for entry in entries {
        if entry.name.ends_with(".log") {
            let content = fs.read_to_string(entry.path)?
            println!("Processing: ${entry.name}")
            analyze_log(content)
        }
    }
    
    Ok(())
}
```

## Types

```windjammer
struct Metadata {
    size: u64,
    is_file: bool,
    is_dir: bool,
    readonly: bool,
    modified: Time,
    created: Time,
}

struct DirEntry {
    path: string,
    name: string,
    is_file: bool,
    is_dir: bool,
}
```

## Error Handling

All operations return `Result<T, Error>` for proper error handling with `?` operator.

Common errors:
- File not found
- Permission denied
- IO errors

```windjammer
match fs.read_to_string("data.txt") {
    Ok(content) => println!("Read ${content.len()} bytes"),
    Err(e) => println!("Error: ${e}"),
}
```

---

**Status**: API Design Complete  
**Implementation**: Pending  
**Rust Deps**: `std::fs` (stdlib)

