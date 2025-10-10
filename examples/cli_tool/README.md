# CLI Tool Example

A concurrent file processor built with Windjammer, demonstrating:

- Command-line argument parsing with decorators
- Concurrent file processing with `go` keyword
- Channels for communication between goroutines
- File I/O with error handling (`?` operator)
- Progress reporting and statistics

## Running

```bash
wj build
cd output
cargo run -- --help
```

## Usage Examples

```bash
# Process a single file
cargo run -- input.txt --uppercase

# Process multiple files with custom output directory
cargo run -- file1.txt file2.txt file3.txt -o processed/

# Process with transformations and verbose output
cargo run -- *.txt --uppercase --reverse -v

# Use more workers for faster processing
cargo run -- *.txt -w 8 -o output/
```

## Features Demonstrated

### Decorators
- `@command` - Define CLI app metadata
- `@arg` - Configure command-line arguments with help text, defaults, and short flags
- `@timing` - Measure function execution time

### Concurrency
- `go { ... }` - Spawn goroutines for concurrent file processing
- Channels (`mpsc`) for collecting results from workers
- Demonstrates Go-style concurrency in Windjammer

### Error Handling
- `Result<T, Error>` return types
- `?` operator for clean error propagation
- Graceful error reporting per file

### Ownership Inference
Notice how we don't need to explicitly specify borrows in most places:
- Function parameters automatically borrowed when not consumed
- Cloning only when needed for goroutines
- Compiler infers the right ownership pattern

## Performance

With 4 workers, this tool can process thousands of files per second, taking full advantage of multi-core CPUs while maintaining Rust's safety guarantees.

