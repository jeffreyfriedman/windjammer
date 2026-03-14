# Windjammer Linter Design

## Purpose

Automatically detect Rust leakage patterns and suggest idiomatic Windjammer alternatives.
Warnings do not block compilation—they guide developers toward better patterns.

## Warning Codes

### W0001: Explicit Ownership
- **Pattern:** `&self`, `&mut self`, `&Camera` in parameters
- **Suggestion:** Use `self`, `Camera` (let compiler infer)
- **Level:** Note (Style)
- **Exception:** Trait implementations (signature must match trait)
- **Exception:** `extern fn` declarations (FFI requires explicit signatures)

### W0002: Explicit .unwrap()
- **Pattern:** `.unwrap()`, `.expect()`
- **Suggestion:** Use `if let Some(...)` or `match`
- **Level:** Warning
- **Note:** `.unwrap()` is a Rust-specific panic convention

### W0003: Explicit .iter()
- **Pattern:** `.iter()`, `.iter_mut()`
- **Suggestion:** Use direct iteration: `for x in collection`
- **Level:** Note (Style)
- **Note:** Windjammer supports direct iteration

### W0004: Explicit Borrowing
- **Pattern:** `&value` in function call arguments
- **Suggestion:** Use `value` (let compiler infer)
- **Level:** Note (Style)
- **Note:** Windjammer infers borrowing automatically

## Configuration

### Enable/Disable

```bash
wj build game.wj              # Linter enabled (default)
wj build game.wj --no-lint    # Disable Rust leakage warnings
```

For the main windjammer CLI (multi-file builds):

```bash
windjammer build path output  # Linter enabled (default)
windjammer build path output --no-lint  # Disable
```

### Library API

```rust
// compiler.rs - enable_lint parameter
windjammer::build_project(path, output, target, true);   // Lint enabled
windjammer::build_project(path, output, target, false);  // Lint disabled
```

## False Positives

### Trait Implementations

```windjammer
impl Iterator for MyType {
    fn next(&mut self) -> Option<Item> {
        // NO WARNING: trait requires explicit &mut self
    }
}
```

**Rule:** Don't warn on trait implementations (signature must match trait).

### Foreign Function Interface

```windjammer
extern fn rust_function(data: &str) {
    // NO WARNING: FFI requires explicit signatures
}
```

**Rule:** Don't warn on `extern fn` declarations.

## Integration

### Compiler Pipeline

1. Parse source → Program AST
2. Check forbidden patterns (.as_str() - hard error)
3. **Run Rust leakage linter** (warnings only)
4. Analyze
5. Codegen

### Output Format

```
note: explicit ownership annotation [W0001]
  --> test.wj:5:15
  = help: use inferred ownership: `self`
  = note: Windjammer infers ownership automatically
```

## Implementation

### Module Structure

```
src/linter/
  mod.rs           # LintCollector, LintDiagnostic, Linter
  rust_leakage.rs  # RustLeakageLinter, W0001-W0004
```

### Key Types

- `RustLeakageLinter` - Walks Program AST, collects diagnostics
- `LintDiagnostic` - Single warning with location, message, suggestion
- `LintCollector` - Aggregates diagnostics

## Future Enhancements

- W0005: Unused variables
- W0006: Unreachable code
- W0007: Missing documentation
- W0008: Performance anti-patterns
- LSP integration for real-time editor warnings

## TDD Tests

Location: `tests/linter_test.rs`

- `test_detect_explicit_self_mut` - W0001
- `test_detect_unwrap` - W0002
- `test_detect_iter` - W0003
- `test_detect_explicit_borrow` - W0004
- `test_no_false_positives_trait_impl` - Trait exception
- `test_no_false_positives_idiomatic` - Clean code
- `test_linter_catches_all_patterns` - Combined
- `test_suggestions_are_helpful` - Suggestion quality
- `test_extern_fn_no_warning` - Extern exception
