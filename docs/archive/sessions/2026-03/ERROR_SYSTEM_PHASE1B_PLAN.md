# Error System Phase 1b: AST Source Tracking - Implementation Plan

## Status: Phase 1a COMPLETE ✅ | Phase 1b READY TO IMPLEMENT

## Phase 1a Completed ✅

**Date:** 2025-11-02  
**Files Created:**
- `src/source_map.rs` - Complete source map data structures
- All tests passing (8 comprehensive tests)
- Serialization/deserialization working
- `CodeGenerator` has `source_map` field ready

## Phase 1b: Add Source Tracking to AST

### Overview

Add source location information to AST nodes so the code generator can populate the source map during transpilation.

### Implementation Steps

#### Step 1: Extend Token with Source Info

**File:** `src/lexer.rs`

Current state:
- Lexer tracks `line` and `column` (lines 158-159)
- Tokens have no location information

**Changes needed:**
```rust
// Add to lexer.rs
pub struct TokenWithLocation {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}

impl Lexer {
    pub fn next_token_with_location(&mut self) -> TokenWithLocation {
        let line = self.line;
        let column = self.column;
        let token = self.next_token();
        TokenWithLocation { token, line, column }
    }
}
```

**Estimated effort:** 2 hours
**Risk:** Low - additive change, doesn't break existing code

---

#### Step 2: Add SourceLocation to AST Nodes

**File:** `src/parser/ast.rs`

**New type:**
```rust
use crate::source_map::Location;

pub type SourceLocation = Option<Location>;
```

**Modify key AST nodes:**

1. **Expression** (most important):
```rust
pub enum Expression {
    Literal {
        value: Literal,
        location: SourceLocation,
    },
    Identifier {
        name: String,
        location: SourceLocation,
    },
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
        location: SourceLocation,
    },
    // ... and so on for all variants
}
```

2. **Statement:**
```rust
pub enum Statement {
    Let {
        pattern: String,
        type_: Option<Type>,
        value: Expression,
        location: SourceLocation,
    },
    // ... for all variants
}
```

3. **Item:**
```rust
pub enum Item {
    Function {
        decl: FunctionDecl,
        location: SourceLocation,
    },
    // ... for all variants
}
```

**Estimated effort:** 8-10 hours (many AST nodes)
**Risk:** Medium - breaks existing code, requires updates throughout parser

---

#### Step 3: Update Parser to Track Locations

**File:** `src/parser_impl.rs`, `src/parser/expression_parser.rs`, etc.

**Changes:**
```rust
impl Parser {
    // Helper to get current location
    fn current_location(&self) -> SourceLocation {
        if self.position < self.tokens.len() {
            Some(Location {
                file: PathBuf::from(&self.filename),
                line: self.tokens[self.position].line,
                column: self.tokens[self.position].column,
            })
        } else {
            None
        }
    }
    
    // Example usage in parse_expression
    fn parse_primary_expression(&mut self) -> Result<Expression, String> {
        let location = self.current_location();
        
        match self.current_token() {
            Token::IntLiteral(n) => {
                let n = *n;
                self.advance();
                Ok(Expression::Literal {
                    value: Literal::Int(n),
                    location,
                })
            }
            // ... etc
        }
    }
}
```

**Estimated effort:** 12-15 hours (update all parse functions)
**Risk:** High - touches most of the parser, potential for bugs

---

#### Step 4: Update CodeGenerator to Use Source Locations

**File:** `src/codegen/rust/generator.rs`

**Changes:**
```rust
impl CodeGenerator {
    fn generate_expression(&mut self, expr: &Expression) -> String {
        // Extract location from expression
        let location = self.get_expression_location(expr);
        
        // Generate the Rust code
        let rust_code = match expr {
            Expression::Literal { value, .. } => self.generate_literal(value),
            // ... existing logic
        };
        
        // Record mapping if we have location info
        if let Some(loc) = location {
            self.record_mapping(&rust_code, &loc);
        }
        
        rust_code
    }
    
    fn record_mapping(&mut self, rust_code: &str, wj_location: &Location) {
        // Calculate Rust line number based on generated output
        let rust_line = self.current_rust_line();
        
        self.source_map.add_mapping(
            PathBuf::from(&self.output_file),
            rust_line,
            0, // column
            wj_location.file.clone(),
            wj_location.line,
            wj_location.column,
        );
    }
    
    fn get_expression_location(&self, expr: &Expression) -> SourceLocation {
        match expr {
            Expression::Literal { location, .. } => location.clone(),
            Expression::Identifier { location, .. } => location.clone(),
            // ... for all variants
        }
    }
}
```

**Estimated effort:** 6-8 hours
**Risk:** Medium - careful tracking needed

---

#### Step 5: Save Source Map with Generated Code

**File:** `src/main.rs` (or wherever compilation happens)

**Changes:**
```rust
fn compile_windjammer_file(
    input_path: &Path,
    output_dir: &Path,
) -> Result<SourceMap> {
    // ... existing compilation logic ...
    
    let (rust_code, source_map) = code_generator.generate_program(&program);
    
    // Write Rust code
    fs::write(rust_output_path, rust_code)?;
    
    // Write source map
    let source_map_path = output_dir.join("source_map.json");
    source_map.save_to_file(&source_map_path)?;
    
    Ok(source_map)
}
```

**Estimated effort:** 2 hours
**Risk:** Low - straightforward integration

---

#### Step 6: Testing

**New test file:** `tests/source_tracking_test.rs`

```rust
#[test]
fn test_source_tracking() {
    let source = r#"
fn main() {
    let x = 42
    println!("Hello")
}
"#;
    
    let program = parse_with_tracking(source, "test.wj").unwrap();
    
    // Verify AST nodes have location info
    let main_fn = &program.items[0];
    assert!(main_fn.location().is_some());
    
    // Verify line numbers
    let location = main_fn.location().unwrap();
    assert_eq!(location.line, 2); // "fn main()" is on line 2
}

#[test]
fn test_source_map_generation() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}
"#;
    
    let (rust_code, source_map) = compile_to_rust(source, "test.wj").unwrap();
    
    // Verify source map has mappings
    assert!(source_map.len() > 0);
    
    // Verify we can look up locations
    let mapping = source_map.lookup(Path::new("output/test.rs"), 3);
    assert!(mapping.is_some());
}
```

**Estimated effort:** 4 hours
**Risk:** Low - validates the implementation

---

### Total Estimated Effort

- Step 1: 2 hours
- Step 2: 10 hours
- Step 3: 15 hours
- Step 4: 8 hours
- Step 5: 2 hours
- Step 6: 4 hours
- **Total: ~40 hours** (1 week of focused work)

### Risks and Mitigations

**High Risk: Breaking Changes to AST**
- **Mitigation:** Implement incrementally, one AST node type at a time
- Start with Expression, then Statement, then Item
- Keep existing code working with `location: None` during transition

**Medium Risk: Parser Complexity**
- **Mitigation:** Add helper functions for common patterns
- Use macros to reduce boilerplate
- Comprehensive testing at each step

**Low Risk: Performance Impact**
- **Mitigation:** Source tracking is optional (can be `None`)
- No impact on release builds if we want to disable it

### Alternative Approach: Minimal Viable Implementation

If full implementation is too large, start with:

1. **Only track function locations** (not every expression)
2. **Coarse-grained mapping** (function level, not statement level)
3. **Gradually expand** as needed

This would reduce effort to ~15 hours while still providing value.

---

## Next Phase: Phase 2

Once Phase 1b is complete, Phase 2 will:
1. Intercept `rustc` JSON output
2. Parse error messages
3. Look up source locations in the source map
4. Display errors with Windjammer file names and line numbers

---

## References

- `docs/ERROR_MAPPING.md` - Overall error system design
- `docs/design/error-mapping.md` - Detailed design doc
- `src/source_map.rs` - Phase 1a implementation (complete)
- Rust compiler book on diagnostics: https://rustc-dev-guide.rust-lang.org/diagnostics.html

---

**Created:** 2025-11-02  
**Author:** AI Assistant  
**Status:** Ready for implementation

