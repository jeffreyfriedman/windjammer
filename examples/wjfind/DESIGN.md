# wjfind - Production CLI Tool Design

**A blazing-fast file search and processing utility built in Windjammer**

---

## 🎯 Goals

1. **Validate Windjammer CLI capabilities** - Real-world command-line tool
2. **Benchmark against ripgrep** - Measure performance vs. industry standard
3. **Showcase stdlib features** - fs, regex, cli, parallel processing
4. **Production-ready** - Error handling, progress bars, colored output

---

## 📋 Features

### Core Search
- [x] Recursive directory traversal
- [x] Regex pattern matching
- [x] File type filtering (--type)
- [x] Ignore patterns (.gitignore support)
- [x] Case-insensitive search (-i)
- [x] Whole word matching (-w)
- [x] Line number display (-n)
- [x] Context lines (-A, -B, -C)

### Performance
- [x] Parallel file processing (rayon-style)
- [x] Memory-mapped files for large files
- [x] Smart buffering
- [x] Early termination on match limit

### Output
- [x] Colored output (match highlighting)
- [x] JSON output (--json)
- [x] Count-only mode (-c)
- [x] Files-with-matches mode (-l)
- [x] Replace mode (--replace)

### User Experience
- [x] Progress bar for large searches
- [x] Human-readable sizes
- [x] Smart defaults (.gitignore respected)
- [x] Helpful error messages

---

## 🏗️ Architecture

```
wjfind/
├── src/
│   ├── main.wj              # CLI entry point
│   ├── search.wj            # Core search logic
│   ├── walker.wj            # Directory traversal
│   ├── matcher.wj           # Pattern matching
│   ├── output.wj            # Output formatting
│   ├── parallel.wj          # Parallel processing
│   └── config.wj            # Configuration
├── benches/
│   └── search_bench.wj      # Performance benchmarks
├── tests/
│   └── integration_test.wj  # Integration tests
└── README.md
```

---

## 🚀 Usage Examples

### Basic Search
```bash
# Find all occurrences of "TODO" in Rust files
wjfind "TODO" --type rust

# Case-insensitive search
wjfind -i "error" src/

# Whole word matching
wjfind -w "fn" src/
```

### Advanced Search
```bash
# Show 3 lines of context
wjfind "panic" -C 3

# Search only in specific file types
wjfind "async" --type rust --type windjammer

# Exclude directories
wjfind "test" --exclude target --exclude node_modules

# Replace mode (dry-run)
wjfind "old_name" --replace "new_name" --dry-run
```

### Output Formats
```bash
# Count matches only
wjfind "error" -c

# List files with matches
wjfind "TODO" -l

# JSON output for scripting
wjfind "error" --json
```

### Performance
```bash
# Parallel search with 8 threads
wjfind "pattern" -j 8

# Memory-mapped mode for huge files
wjfind "pattern" --mmap

# Limit results
wjfind "pattern" --max-count 100
```

---

## ⚡ Performance Targets

**Goal**: Within 10% of ripgrep performance

| Benchmark | ripgrep | wjfind (target) | Notes |
|-----------|---------|-----------------|-------|
| **Small codebase** (1K files) | ~50ms | ~55ms | Startup overhead acceptable |
| **Large codebase** (100K files) | ~2s | ~2.2s | Parallel processing critical |
| **Huge file** (1GB) | ~500ms | ~550ms | Memory-mapped I/O |
| **Regex complexity** | Varies | Within 10% | Use `regex` crate |

---

## 🧪 Testing Strategy

### Unit Tests
- Pattern matching edge cases
- Directory traversal with symlinks
- Ignore pattern parsing
- Output formatting

### Integration Tests
- End-to-end search scenarios
- Large file handling
- Error conditions
- Cross-platform compatibility

### Benchmarks
- Compare against ripgrep
- Measure parallel scaling
- Profile memory usage
- Test regex performance

---

## 📦 Dependencies (via Windjammer stdlib)

```windjammer
use std.fs       // File system operations
use std.regex    // Pattern matching
use std.cli      // Argument parsing
use std.parallel // Parallel processing
use std.io       // Buffered I/O
use std.path     // Path manipulation
use std.env      // Environment variables
```

**No external crates needed!** Everything through Windjammer stdlib.

---

## 🎨 Output Format

### Default (colored)
```
src/main.wj
  42: fn main() {
  43:     let pattern = "TODO";  // ← TODO highlighted in red
  44:     search(pattern);
  45: }

src/search.wj
  15: // TODO: Optimize this
```

### JSON
```json
{
  "matches": [
    {
      "file": "src/main.wj",
      "line": 43,
      "column": 25,
      "text": "    let pattern = \"TODO\";",
      "match": "TODO"
    }
  ],
  "stats": {
    "files_searched": 150,
    "matches_found": 23,
    "duration_ms": 45
  }
}
```

---

## 🔧 Implementation Plan

### Phase 1: Core (Week 1)
- [x] CLI argument parsing
- [x] Basic directory traversal
- [x] Simple pattern matching
- [x] Colored output
- [ ] .gitignore support

### Phase 2: Performance (Week 2)
- [ ] Parallel file processing
- [ ] Memory-mapped I/O
- [ ] Smart buffering
- [ ] Benchmark against ripgrep

### Phase 3: Features (Week 2-3)
- [ ] Context lines (-A, -B, -C)
- [ ] Replace mode
- [ ] JSON output
- [ ] Progress bar

### Phase 4: Polish (Week 3)
- [ ] Error handling
- [ ] Cross-platform testing
- [ ] Documentation
- [ ] Release binary

---

## 📊 Success Metrics

1. **Performance**: Within 10% of ripgrep on standard benchmarks
2. **Usability**: Intuitive CLI matching ripgrep conventions
3. **Reliability**: 100% test coverage on core logic
4. **Portability**: Works on Linux, macOS, Windows

---

## 🎯 Learnings for Windjammer

This project will validate:
- ✅ CLI parsing ergonomics
- ✅ File I/O performance
- ✅ Regex performance
- ✅ Parallel processing APIs
- ✅ Error handling patterns
- ✅ Cross-platform compatibility
- ✅ Binary distribution

---

*Design created: October 12, 2025*  
*Target: 3 weeks for production-ready v1.0*

