# Windjammer Language Specification Tests

## Purpose

These tests serve three critical purposes:

1. **Regression Prevention**: Ensure compiler changes don't break existing functionality
2. **Refactoring Safety**: Enable safe refactoring of large compiler files
3. **Formal Specification**: Document the language behavior for future spec publication

## Test Categories

### 1. Syntax and Parsing (`tests/parser/`)

**Core Syntax**:
- `basic_types.wj` - Primitive types (int, f32, bool, string)
- `struct_syntax.wj` - Struct declarations with fields
- `enum_syntax.wj` - Enum declarations with variants
- `trait_syntax.wj` - Trait declarations
- `impl_syntax.wj` - Impl blocks
- `function_syntax.wj` - Function declarations
- `module_syntax.wj` - Module (mod) declarations

**Advanced Syntax**:
- `generics.wj` - Generic type parameters and bounds
- `where_clauses.wj` - Where clause syntax
- `pattern_matching.wj` - Match expressions and patterns
- `destructuring.wj` - Tuple and struct destructuring
- `closures.wj` - Closure syntax and capture

**Optional Syntax**:
- `optional_semicolons.wj` - Semicolon rules (optional for statements, required for associated types)
- `implicit_returns.wj` - Implicit return values

### 2. Type System (`tests/types/`)

**Type Inference**:
- `type_inference_basic.wj` - Basic type inference
- `type_inference_generics.wj` - Generic type inference
- `type_inference_closures.wj` - Closure type inference

**Type Checking**:
- `type_checking_basic.wj` - Basic type compatibility
- `type_checking_generics.wj` - Generic type constraints
- `type_checking_traits.wj` - Trait bounds

### 3. Ownership and Borrowing (`tests/ownership/`)

**Ownership Inference**:
- `ownership_inference_params.wj` - Parameter ownership inference (owned, borrowed, mutable)
- `ownership_inference_self.wj` - Self parameter inference (&self, &mut self, self)
- `ownership_inference_return.wj` - Return type ownership inference

**Builder Pattern**:
- `builder_pattern.wj` - Builder pattern with self-returning methods
- `builder_pattern_chaining.wj` - Method chaining

**Field Access**:
- `field_access_simple.wj` - Simple field access
- `field_access_mutation.wj` - Field mutation detection
- `field_access_constructor.wj` - Constructor vs method distinction

### 4. Code Generation (`tests/codegen/`)

**Statement Generation**:
- `implicit_return_after_let.wj` - **[BUG]** Implicit return after let statements
- `multiple_lets.wj` - Multiple consecutive let statements
- `statement_ordering.wj` - Statement order preservation

**Expression Generation**:
- `operator_precedence.wj` - Operator precedence and associativity
- `method_calls.wj` - Method call generation
- `field_access.wj` - Field access generation

**Auto-Optimizations**:
- `auto_clone.wj` - Automatic .clone() insertion
- `auto_mut.wj` - Automatic mut inference for local variables
- `auto_derive.wj` - Automatic #[derive(...)] for simple types

**Trait Implementations**:
- `trait_impl.wj` - Basic trait implementation
- `trait_impl_generics.wj` - Generic trait implementation
- `trait_impl_operators.wj` - Operator trait implementations (Add, Sub, etc.)

### 5. FFI and Interop (`tests/ffi/`)

**FFI Declarations**:
- `extern_fn_basic.wj` - Basic extern function declarations
- `extern_fn_generic.wj` - **[NEW]** Generic extern functions
- `mod_ffi.wj` - FFI declarations in mod blocks

**C Interop**:
- `c_types.wj` - C-compatible types
- `c_structs.wj` - #[repr(C)] structs

**Rust Interop**:
- `rust_std_types.wj` - Using Rust std types (String, Vec, etc.)
- `rust_traits.wj` - Implementing Rust traits

### 6. Advanced Features (`tests/advanced/`)

**Decorators**:
- `decorator_test.wj` - @test decorator
- `decorator_async.wj` - @async decorator
- `decorator_export.wj` - @export decorator

**Async/Await**:
- `async_functions.wj` - Async function declarations
- `await_expressions.wj` - Await syntax

**Error Handling**:
- `result_type.wj` - Result<T, E> usage
- `option_type.wj` - Option<T> usage
- `error_propagation.wj` - ? operator

### 7. Integration Tests (`tests/integration/`)

**Complete Programs**:
- `hello_world.wj` - Minimal program
- `cli_app.wj` - Command-line app with args
- `web_server.wj` - Basic HTTP server
- `game_minimal.wj` - Minimal game with GameLoop

## Test Infrastructure

### Test Runner

```rust
// tests/compiler_test.rs

fn test_codegen(test_name: &str) {
    // 1. Compile .wj file
    // 2. Compare with .expected.rs
    // 3. Verify Rust compilation succeeds
}

fn test_parse(test_name: &str) {
    // 1. Parse .wj file
    // 2. Verify AST structure
    // 3. Verify no parse errors
}

fn test_error(test_name: &str) {
    // 1. Parse/compile .wj file
    // 2. Verify expected error is produced
    // 3. Verify error message quality
}
```

### Test Format

Each test has two files:

```
tests/codegen/
  ├── implicit_return_after_let.wj         # Input
  └── implicit_return_after_let.expected.rs # Expected output
```

### Negative Tests

Tests that should FAIL to compile:

```
tests/errors/
  ├── type_mismatch.wj           # Should error: type mismatch
  ├── undefined_variable.wj      # Should error: undefined variable
  ├── invalid_trait_bound.wj     # Should error: trait not satisfied
  └── borrow_checker.wj          # Should error: borrow conflicts
```

## Coverage Goals

### Phase 1: Critical Path (Immediate)
- ✅ Implicit returns (bug we're fixing now)
- ⏳ Ownership inference (builder pattern, constructors)
- ⏳ Auto mut inference
- ⏳ Auto derive

### Phase 2: Core Features (This Week)
- Trait implementations
- Generic functions
- FFI declarations
- Module system

### Phase 3: Advanced Features (Next Week)
- Decorators
- Async/await
- Error handling
- Optimizations

### Phase 4: Integration (Ongoing)
- Complete programs
- Real-world use cases
- Performance benchmarks

## Benefits for Spec Publication

When we publish the formal Windjammer specification:

1. **Executable Spec**: Tests demonstrate exact behavior
2. **Examples**: Each test is a documented example
3. **Edge Cases**: Negative tests document what's NOT allowed
4. **Versioning**: Track language evolution through test history

## Implementation Priority

**HIGH**: These tests are critical before we refactor the compiler. The `generator.rs` file is already 4,827 lines and needs to be broken up into:
- `generator/mod.rs` - Main orchestration
- `generator/functions.rs` - Function code generation
- `generator/types.rs` - Type and struct generation
- `generator/expressions.rs` - Expression generation
- `generator/statements.rs` - Statement generation
- `generator/traits.rs` - Trait and impl generation
- `generator/optimizations.rs` - Optimization passes

**Without tests, refactoring is dangerous!**

## Action Items

1. Create test infrastructure (this file + compiler_test.rs)
2. Add tests for critical bugs (implicit_return_after_let)
3. Add tests for recent features (auto_mut, auto_derive, generic_extern_fn)
4. Expand coverage to all compiler phases
5. Set up CI to run tests automatically
6. Use tests as safety net for refactoring

## Success Criteria

- ✅ 100+ codegen tests covering all language features
- ✅ 50+ parser tests covering all syntax
- ✅ 25+ error tests covering failure modes
- ✅ All tests pass on every commit
- ✅ Tests serve as executable specification
- ✅ Safe to refactor compiler into smaller modules

**Tests = Safety net + Documentation + Specification**


















