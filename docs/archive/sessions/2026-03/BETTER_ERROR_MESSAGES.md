# Better Error Messages (P3)

**Context-aware, helpful error reporting for Windjammer compiler and runtime.**

**Status:** Implemented (2026-03-14)  
**Design Reference:** Part 5 of `GAME_ENGINE_IMPROVEMENTS_DESIGN.md`

---

## Overview

This document describes the enhanced error message system that provides:

1. **Compiler**: Rust-style error context with source lines, carets, and fix suggestions
2. **Runtime**: Game-developer-friendly panic formatting with troubleshooting tips
3. **GPU**: Contextual error messages for shader/buffer issues
4. **Suggestions**: Actionable fix hints for common error codes

---

## Components

### 1. ErrorContext (`windjammer/src/error/context_builder.rs`)

Enhanced compiler error display with:

- **Source location**: File, line, column
- **Source line**: The actual line of code
- **Caret**: Points to the error column
- **Error type**: E.g., E0425, WJ0001
- **Help**: Optional fix suggestion
- **Note**: Optional additional context

**Example output:**

```
error[E0425]: cannot find value `undefined_var`
  --> game.wj:42:15
   |
 42 |     let x = undefined_var
   |               ^
   |
help: did you mean `defined_var`?
```

**Usage:**

```rust
use windjammer::error::ErrorContext;

let ctx = ErrorContext {
    file: "game.wj".to_string(),
    line: 42,
    column: 15,
    source_line: "    let x = undefined_var".to_string(),
    error_type: "E0425".to_string(),
    message: "cannot find value `undefined_var`".to_string(),
    help: Some("did you mean `defined_var`?".to_string()),
    note: None,
};
eprintln!("{}", ctx.format());
```

### 2. suggest_fix (`windjammer/src/error/suggestions.rs`)

Maps error codes to actionable fix suggestions.

**Supported error codes:**

| Code | Description |
|------|-------------|
| E0425, WJ0001 | Variable not found |
| E0308, WJ0003 | Type mismatch |
| E0404, WJ0004 | Expected type |
| E0583, WJ0005 | Scope/block error |
| E0061, WJ0006 | Argument count mismatch |
| E0063, WJ0007 | Missing struct field |
| E0382, WJ0008 | Use of moved value |
| E0596, WJ0009 | Borrowing error |
| E0599, WJ0002 | Method/function not found |
| E0601 | Shader/GPU type mismatch |

**Usage:**

```rust
use windjammer::error::suggest_fix;

if let Some(suggestion) = suggest_fix("E0425", "undefined_var") {
    eprintln!("help: {}", suggestion);
}
```

### 3. RuntimeErrorHandler (`windjammer-runtime-host/src/error_handler.rs`)

Custom panic hook that formats runtime errors with:

- **Location**: File, line, column from panic location
- **Message**: The panic payload
- **Troubleshooting tips**: Shader bindings, buffer sizes, FRAME_DEBUG, RUST_BACKTRACE

**Example output:**

```
🔴 Runtime Error:
  Location: src/gpu_compute.rs:123:45
  Message: Buffer validation failed

💡 Troubleshooting:
  1. Check shader bindings match host code
  2. Verify buffer sizes are correct
  3. Enable FRAME_DEBUG=1 for diagnostics
  4. Run with RUST_BACKTRACE=1 for full trace
```

**Integration:** Call `RuntimeErrorHandler::install()` at the start of `main()`:

```rust
fn main() {
    env_logger::init();
    let _ = runtime::error_handler::RuntimeErrorHandler::install();
    // ... rest of main
}
```

### 4. gpu_error_context (`windjammer-runtime-host/src/gpu_compute.rs`)

Formats GPU errors with operation context and common causes:

```rust
eprintln!("{}", gpu_error_context("gpu_dispatch_compute", "Buffer validation failed"));
```

**Output includes:**

- Operation name
- Raw error message
- Common causes (binding mismatch, uniform size, workgroup size, bounds)
- HOT_RELOAD=1 suggestion for development

---

## Integration Points

### Compiler

- **error_mapper.rs**: Uses `suggest_fix()` when rustc reports an error with a code but no help messages. Adds the suggestion to the diagnostic's help array.
- **error module**: Exports `ErrorContext`, `suggest_fix` for use by CLI, LSP, and other tools.

### Runtime

- **windjammer-game main.rs**: Installs `RuntimeErrorHandler` at startup
- **breach-protocol runtime_host main.rs**: Installs `RuntimeErrorHandler` at startup

### GPU

- **gpu_compute.rs**: `gpu_error_context()` available for use when reporting wgpu/shader errors. Call sites can be added where errors are caught (e.g., shader compilation, buffer creation).

---

## Testing

### Compiler (windjammer crate)

```bash
cd windjammer
cargo test --lib error::
```

**Test counts:**

- `context_builder`: 6 tests
- `suggestions`: 12 tests
- `error::tests`: 1 test (CompileError display)

### Runtime (windjammer-runtime-host)

```bash
cd windjammer-game/windjammer-runtime-host
cargo test error_handler::
cargo test gpu_error_context_tests::
```

**Test counts:**

- `error_handler`: 3 tests
- `gpu_error_context_tests`: 3 tests

---

## Future Enhancements

1. **ErrorContext in compiler**: Use `ErrorContext::format()` when emitting compiler errors (parser, analyzer) instead of/in addition to `CompileError`.
2. **GPU error integration**: Call `gpu_error_context()` in `gpu_load_compute_shader` and other FFI entry points when wgpu returns errors.
3. **More suggestions**: Expand `suggest_fix` for additional error codes as they are encountered.
4. **Context-aware suggestions**: Use the `context` parameter in `suggest_fix` for variable-name-specific hints (e.g., "did you mean `foo`?").
5. **LSP integration**: Use `ErrorContext` and `suggest_fix` in the language server for rich diagnostics.

---

## Philosophy Alignment

- **"Compiler Does the Hard Work"**: Error messages do the work of explaining what went wrong and how to fix it.
- **"No Workarounds, Only Proper Fixes"**: Suggestions point to correct solutions, not hacks.
- **"Safety Without Ceremony"**: Helpful errors reduce the burden of debugging.

---

*Part of the Game Engine Improvements initiative. See `GAME_ENGINE_IMPROVEMENTS_DESIGN.md` for the full strategic plan.*
