# Error Mapping System Design

## Problem Statement

**Current Issue**: Users write Windjammer code but receive Rust compiler errors pointing to generated Rust code.

**Example**:
```windjammer
// user.wj:10
fn greet(name: string) {
    println!("Hello, ${name}!")
}
```

**Current Error**:
```
error[E0308]: mismatched types
 --> output/user.rs:3
  |
3 | fn greet(name: &String) {
  |                ^^^^^^^ expected `&str`, found `&String`
```

**Problem**: User doesn't know what `output/user.rs:3` means in terms of their Windjammer code!

---

## Solution: Three-Part System

### Part 1: Source Maps

**Concept**: Map every line in generated Rust back to source Windjammer line.

**Implementation**:
```rust
// In codegen.rs
struct SourceMap {
    mappings: Vec<Mapping>,
}

struct Mapping {
    rust_file: String,
    rust_line: usize,
    wj_file: String,
    wj_line: usize,
    wj_column: usize,
}
```

**Generation**:
```rust
impl CodeGenerator {
    fn generate_function(&mut self, func: &Function) {
        // Track source location
        self.add_mapping(
            rust_line: self.current_rust_line,
            wj_file: func.source_file,
            wj_line: func.source_line,
        );
        
        // Generate code...
    }
}
```

**Output**: `output/source_map.json`
```json
{
  "mappings": [
    {
      "rust_file": "output/user.rs",
      "rust_line": 3,
      "wj_file": "user.wj",
      "wj_line": 10,
      "wj_column": 1
    }
  ]
}
```

---

### Part 2: Error Interception & Translation

**Concept**: Intercept Rust compiler output and translate it to Windjammer locations.

**Architecture**:
```
Windjammer Build Process:
1. Transpile .wj → .rs (generate source map)
2. Run `cargo build --message-format=json`
3. Intercept JSON error messages
4. Translate using source map
5. Display Windjammer-friendly errors
```

**Implementation**:
```rust
// src/error_translator.rs
pub struct ErrorTranslator {
    source_map: SourceMap,
}

impl ErrorTranslator {
    pub fn translate(&self, rust_error: RustError) -> WindjammerError {
        // Look up Windjammer location
        let wj_location = self.source_map.lookup(
            rust_error.file,
            rust_error.line
        )?;
        
        // Translate error message
        let wj_message = self.translate_message(rust_error.message);
        
        WindjammerError {
            file: wj_location.file,
            line: wj_location.line,
            column: wj_location.column,
            message: wj_message,
            severity: rust_error.severity,
        }
    }
    
    fn translate_message(&self, rust_msg: &str) -> String {
        // Replace Rust terminology with Windjammer terminology
        rust_msg
            .replace("&String", "borrowed string")
            .replace("&str", "string slice")
            .replace("&i64", "borrowed int")
            // ... more translations
    }
}
```

---

### Part 3: User-Friendly Error Display

**Concept**: Show errors in Windjammer context with helpful suggestions.

**Example Translation**:

**Rust Error**:
```
error[E0308]: mismatched types
 --> output/user.rs:3
  |
3 | fn greet(name: &String) {
  |                ^^^^^^^ expected `&str`, found `&String`
```

**Windjammer Error**:
```
error: type mismatch
  --> user.wj:10:13
   |
10 | fn greet(name: string) {
   |             ^^^^^^ expected string slice, found borrowed string
   |
help: Windjammer inferred this as a borrowed parameter.
      The function is trying to use it as a string slice.
      
suggestion: Consider using string slice operations:
   |
10 | fn greet(name: string) {
11 |     println!("Hello, {}!", name.as_str())
   |
```

---

## Error Message Translation Dictionary

### Rust → Windjammer Type Names

| Rust Type | Windjammer Display |
|-----------|-------------------|
| `&String` | `borrowed string` or `&string` |
| `&str` | `string slice` |
| `&i64` | `borrowed int` or `&int` |
| `&mut T` | `mutable borrow of T` |
| `Vec<T>` | `Vec<T>` (same) |
| `Option<T>` | `Option<T>` (same) |
| `Result<T, E>` | `Result<T, E>` (same) |

### Common Error Patterns

**Pattern 1: Type Mismatch**
```
Rust: expected `&i64`, found integer
Windjammer: expected borrowed int, found int value
Suggestion: The compiler inferred this parameter needs borrowing. Pass &value instead.
```

**Pattern 2: Lifetime Issues**
```
Rust: borrowed value does not live long enough
Windjammer: value doesn't live long enough
Explanation: This value is borrowed but goes out of scope before the borrow ends.
Suggestion: Consider cloning the value or restructuring ownership.
```

**Pattern 3: Mutability Issues**
```
Rust: cannot borrow as mutable
Windjammer: cannot modify this value
Explanation: The compiler inferred this as immutable. Use 'let mut' to make it mutable.
```

---

## Implementation Plan

### Phase 1: Basic Source Mapping
- [ ] Add source location tracking to AST nodes
- [ ] Generate line mappings during code generation
- [ ] Write source map to JSON file
- [ ] Create lookup utility

**Files to Modify**:
- `src/parser.rs` - Add source locations to AST
- `src/codegen.rs` - Track line mappings
- Create `src/source_map.rs`

### Phase 2: Error Interception
- [ ] Capture `cargo build` JSON output
- [ ] Parse Rust compiler messages
- [ ] Look up source locations
- [ ] Basic error translation

**Files to Create**:
- `src/error_translator.rs`
- Update `src/main.rs` to use error translator

### Phase 3: Error Translation
- [ ] Build Rust→Windjammer terminology dictionary
- [ ] Translate common error patterns
- [ ] Add helpful suggestions
- [ ] Context-aware messages

### Phase 4: Enhanced Display
- [ ] Colorized output
- [ ] Code snippets with highlighting
- [ ] Multi-line context
- [ ] Inline suggestions

---

## Example Implementation

### 1. Add Source Locations to AST

```rust
// src/parser.rs
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
    pub location: SourceLocation,  // Add this
}
```

### 2. Track During Code Generation

```rust
// src/codegen.rs
impl CodeGenerator {
    fn generate_function(&mut self, func: &FunctionDecl) {
        let rust_line = self.current_line;
        
        // Record mapping
        self.source_map.add_mapping(Mapping {
            rust_file: self.output_file.clone(),
            rust_line,
            wj_file: func.location.file.clone(),
            wj_line: func.location.line,
            wj_column: func.location.column,
        });
        
        // Generate code...
        let output = format!("fn {}(", func.name);
        self.emit_line(&output);
    }
    
    fn emit_line(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
        self.current_line += 1;
    }
}
```

### 3. Intercept and Translate Errors

```rust
// src/main.rs
fn build_and_report_errors(files: Vec<String>) -> Result<(), Error> {
    // 1. Transpile
    let (rust_files, source_map) = transpile(files)?;
    
    // 2. Run cargo with JSON output
    let output = Command::new("cargo")
        .args(&["build", "--message-format=json"])
        .current_dir("output")
        .output()?;
    
    // 3. Parse and translate errors
    let translator = ErrorTranslator::new(source_map);
    let rust_errors = parse_cargo_output(&output.stdout)?;
    
    for rust_error in rust_errors {
        let wj_error = translator.translate(rust_error)?;
        display_error(&wj_error);
    }
    
    Ok(())
}
```

---

## Error Display Format

### Standard Format
```
error: <short description>
  --> <file>:<line>:<column>
   |
<line_num> | <code line>
           | <highlight>
           |
help: <suggestion>
```

### Example
```
error: type mismatch in function call
  --> main.wj:15:21
   |
15 |     let result = double(5)
   |                         ^ expected &int, found int
   |
help: function 'double' expects a borrowed int
      
suggestion: pass a reference:
   |
15 |     let result = double(&5)
   |                         +
```

---

## Benefits

**For Users**:
- ✅ Errors point to their actual code
- ✅ Understandable terminology
- ✅ Helpful suggestions
- ✅ Learn Windjammer, not Rust internals

**For Debugging**:
- ✅ Source maps enable debugging tools
- ✅ Stack traces map correctly
- ✅ IDE integration possible

**For Adoption**:
- ✅ Better developer experience
- ✅ Lower learning curve
- ✅ More professional tooling

---

## Challenges

**Challenge 1: Mapping Accuracy**
- Generated code may not be 1:1 with source
- Macros and transformations complicate mapping

**Solution**: Track at statement level, not just line level

**Challenge 2: Error Message Quality**
- Rust errors are complex
- Hard to translate all cases

**Solution**: Start with common errors, iteratively improve

**Challenge 3: Performance**
- Adding mapping overhead
- Error translation takes time

**Solution**: Only generate maps in debug mode, cache translations

---

## Future Enhancements

### Phase 5: IDE Integration
- [ ] LSP error reporting
- [ ] Inline error display
- [ ] Quick fixes

### Phase 6: Advanced Features
- [ ] Rust panic traces → Windjammer stack traces
- [ ] Debugger integration (LLDB/GDB)
- [ ] Performance profiler integration

---

## Success Metrics

**Must Have (v0.2)**:
- ✅ Basic source mapping working
- ✅ Errors point to Windjammer files
- ✅ Common error patterns translated

**Should Have (v0.3)**:
- ✅ Helpful suggestions
- ✅ Context-aware messages
- ✅ Colorized output

**Nice to Have (v1.0)**:
- ✅ IDE integration
- ✅ Stack trace mapping
- ✅ Debugger integration

---

*Status: Design Complete*  
*Priority: P0 (Critical for usability)*  
*Estimated Effort: 1-2 weeks*  
*Next Step: Implement Phase 1 (source mapping)*

