# Error Mapping: Rust → Windjammer

## Problem

When Windjammer code is transpiled to Rust, any compilation errors from `rustc` refer to:
- Generated Rust code (e.g., `build_output/main.rs:42`)
- Rust syntax and terminology
- Line numbers that don't match the original `.wj` file

This creates a poor developer experience - users write Windjammer but get Rust errors!

## Solution Overview

We need a **Source Map** system that:
1. Tracks the mapping between generated Rust lines and original Windjammer lines
2. Intercepts `rustc` errors and translates them back to Windjammer context
3. Rewrites error messages to use Windjammer terminology

## Architecture

### Phase 1: Source Map Generation (MVP)

```rust
pub struct SourceMap {
    // Map: (rust_file, rust_line) -> (wj_file, wj_line)
    mappings: HashMap<(PathBuf, usize), (PathBuf, usize)>,
}
```

During code generation, record every time we emit a line:
```rust
impl CodeGenerator {
    fn emit_line(&mut self, rust_line: String, wj_source: &SourceLocation) {
        self.output.push(rust_line);
        self.source_map.add_mapping(
            self.output.len(),
            wj_source.file.clone(),
            wj_source.line,
        );
    }
}
```

### Phase 2: Error Interception

After running `rustc`, parse its stderr output:
```rust
fn compile_rust_and_map_errors(
    rust_dir: &Path,
    source_map: &SourceMap,
) -> Result<CompileResult> {
    let output = Command::new("rustc")
        .args(&["--json", "diagnostics"])
        .output()?;
    
    let errors = parse_rustc_json(output.stderr)?;
    let mapped_errors = errors.iter()
        .map(|err| map_error(err, source_map))
        .collect();
    
    Ok(CompileResult { errors: mapped_errors })
}
```

### Phase 3: Error Message Translation

Common translations:
- `cannot find type` → "Type not found" 
- `expected &str, found &String` → "Type mismatch (use string instead)"
- `trait bounds not satisfied` → "Missing trait implementation"
- `cannot move out of` → "Ownership error (variable was moved)"

```rust
fn translate_error_message(rust_msg: &str) -> String {
    // Pattern matching for common Rust error patterns
    match rust_msg {
        msg if msg.contains("cannot find type") => 
            format!("Type not found: {}", extract_type_name(msg)),
        msg if msg.contains("expected") && msg.contains("found") =>
            format!("Type mismatch: {}", simplify_type_error(msg)),
        // ... more patterns
        _ => rust_msg.to_string() // Fallback to original
    }
}
```

## Implementation Plan

### Step 1: Add SourceLocation to Parser (DONE)
Already exists in AST nodes via the parser position tracking.

### Step 2: Build SourceMap During Codegen
- Add `source_map: SourceMap` field to `CodeGenerator`
- Track current AST node being generated
- Record mapping for each emitted line

### Step 3: Capture and Parse rustc Output
- Run `cargo build --message-format=json`
- Parse JSON diagnostic messages
- Extract file, line, column, message

### Step 4: Map Errors Back
- Look up Rust location in source map
- Find corresponding Windjammer location
- Rewrite error with Windjammer context

### Step 5: Pretty Print Errors
```
Error in examples/hello/main.wj:12:5
  |
12|     let x: int = "hello"
  |         ^ Type mismatch: expected int, found string
  |
  = help: Convert the string to an integer using parse()
```

## Future Enhancements

### Enhanced Mappings
Track not just line numbers but also:
- Column offsets
- Expression spans (start/end)
- Multiple Windjammer lines → single Rust line

### Contextual Help
Provide Windjammer-specific suggestions:
```
error: Missing @ before decorator
  |
15|   wasm_bindgen
  |   ^^^^^^^^^^^^ expected '@' prefix
  |
  = help: Use @wasm_bindgen or @export
```

### IDE Integration
Export source maps in a standard format (e.g., JSON Source Maps v3) for LSP/editor integration.

## Benefits

1. **Immediate UX Win**: Users see errors in their own code, not generated code
2. **Reduced Confusion**: Windjammer terminology instead of Rust jargon
3. **Faster Debugging**: Correct line numbers make fixing errors trivial
4. **Professional Feel**: Matches the UX of mature languages

## Example

**Before** (current):
```
error[E0308]: mismatched types
  --> build_output/main.rs:42:14
   |
42 |     let x: i64 = "hello";
   |            ---   ^^^^^^^ expected `i64`, found `&str`
```

**After** (with mapping):
```
Error in examples/hello/main.wj:10:5
  |
10|     let x: int = "hello"
  |                  ^^^^^^^ Type mismatch: expected int, found string
  |
  = help: Use .parse() to convert a string to an integer
```

Much better developer experience!
