# Windjammer Test Coverage Plan

## Goal: 100% Test Coverage

This plan organizes tests to achieve comprehensive coverage of the Windjammer compiler.
Tests serve as:
1. **Regression prevention** - Catch bugs before they reach users
2. **Documentation** - Living documentation of language behavior
3. **Specification** - Foundation for the Windjammer Language Spec
4. **Refactoring safety** - Confidence to improve the codebase

## Current Status

- **Test files**: 45+
- **Total tests**: 734
- **Coverage**: Good parser coverage, expanding analyzer/codegen

### Completed Test Categories
- ✅ Lexer tests: 50 tests
- ✅ Parser expression tests: 59 tests
- ✅ Parser statement tests: 51 tests
- ✅ Parser item tests: 47 tests
- ✅ Parser type tests: 40 tests
- ✅ Ownership inference tests: 16 tests
- ✅ String handling tests: 14 tests
- ✅ Method call tests: 4 tests
- ✅ Borrowed string tests: 3 tests
- ✅ Struct constructor tests: 4 tests
- ✅ Iterator clone tests: 3 tests
- Plus existing integration/end-to-end tests

## Test Categories

### 1. Lexer Tests (`lexer_tests.rs`)
- [ ] Keywords: `fn`, `let`, `struct`, `enum`, `impl`, `trait`, `pub`, `use`, etc.
- [ ] Operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
- [ ] Assignment: `=`, `+=`, `-=`, `*=`, `/=`
- [ ] Delimiters: `(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`, `::`
- [ ] Literals: integers, floats, strings, booleans, chars
- [ ] String interpolation: `${}` syntax
- [ ] Comments: `//`, `/* */`, `///` doc comments
- [ ] Whitespace and newline handling
- [ ] Error recovery for malformed tokens

### 2. Parser Tests

#### 2.1 Expression Tests (`parser_expression_tests.rs`)
- [ ] Literals: `42`, `3.14`, `"hello"`, `true`, `'c'`
- [ ] Identifiers: `foo`, `_bar`, `snake_case`
- [ ] Binary operations: `a + b`, `x * y`, `m && n`
- [ ] Unary operations: `-x`, `!flag`, `*ptr`
- [ ] Function calls: `foo()`, `bar(1, 2)`, `baz(a: 1)`
- [ ] Method calls: `obj.method()`, `x.chain().call()`
- [ ] Field access: `obj.field`, `a.b.c`
- [ ] Index access: `arr[0]`, `map[key]`
- [ ] Struct literals: `Point { x: 1, y: 2 }`
- [ ] Array literals: `[1, 2, 3]`
- [ ] Tuple expressions: `(a, b, c)`
- [ ] Range expressions: `0..10`, `1..=5`
- [ ] Closure expressions: `|x| x + 1`, `|a, b| { a + b }`
- [ ] If expressions: `if cond { a } else { b }`
- [ ] Match expressions: `match x { ... }`
- [ ] Block expressions: `{ stmt; expr }`
- [ ] Reference expressions: `&x`, `&mut y`
- [ ] Cast expressions: `x as i32`
- [ ] Operator precedence: `a + b * c` vs `(a + b) * c`

#### 2.2 Statement Tests (`parser_statement_tests.rs`)
- [ ] Let statements: `let x = 1`, `let mut y = 2`
- [ ] Let with type: `let x: i32 = 1`
- [ ] Let with patterns: `let (a, b) = tuple`
- [ ] Assignment: `x = 1`, `a.b = 2`, `arr[0] = 3`
- [ ] Compound assignment: `x += 1`, `y *= 2`
- [ ] Expression statements: `foo();`, `println!("hi")`
- [ ] Return statements: `return`, `return x`
- [ ] If statements: `if cond { ... }`, `if cond { ... } else { ... }`
- [ ] Else-if chains: `if a { } else if b { } else { }`
- [ ] While loops: `while cond { ... }`
- [ ] For loops: `for x in iter { ... }`
- [ ] For with patterns: `for (i, x) in iter.enumerate() { ... }`
- [ ] Match statements: `match x { pat => { ... } }`
- [ ] Break/Continue: `break`, `continue`

#### 2.3 Item Tests (`parser_item_tests.rs`)
- [ ] Function declarations: `fn foo() { }`
- [ ] Function with params: `fn foo(x: i32, y: string) { }`
- [ ] Function with return type: `fn foo() -> i32 { 42 }`
- [ ] Generic functions: `fn foo<T>(x: T) { }`
- [ ] Async functions: `async fn foo() { }`
- [ ] Extern functions: `extern fn rust_fn(x: i32) -> i32`
- [ ] Struct declarations: `struct Point { x: i32, y: i32 }`
- [ ] Tuple structs: `struct Color(i32, i32, i32)`
- [ ] Unit structs: `struct Empty`
- [ ] Enum declarations: `enum Option<T> { Some(T), None }`
- [ ] Impl blocks: `impl Point { fn new() -> Self { } }`
- [ ] Trait impl: `impl Display for Point { }`
- [ ] Trait declarations: `trait Drawable { fn draw(&self); }`
- [ ] Use statements: `use std::fs`, `use crate::module::*`
- [ ] Mod declarations: `mod foo`, `mod bar { }`
- [ ] Decorators: `@auto`, `@derive(...)`, `@component`

#### 2.4 Type Tests (`parser_type_tests.rs`)
- [ ] Primitive types: `i32`, `f64`, `bool`, `string`, `char`
- [ ] Reference types: `&T`, `&mut T`
- [ ] Array types: `[T; N]`, `[T]`
- [ ] Slice types: `&[T]`
- [ ] Tuple types: `(A, B, C)`
- [ ] Option type: `Option<T>`
- [ ] Result type: `Result<T, E>`
- [ ] Vec type: `Vec<T>`
- [ ] HashMap type: `HashMap<K, V>`
- [ ] Generic types: `Box<T>`, `Arc<T>`
- [ ] Function types: `fn(A) -> B`
- [ ] Nested generics: `Vec<Option<T>>`
- [ ] Where clauses: `where T: Clone`

### 3. Analyzer Tests

#### 3.1 Ownership Inference Tests (`analyzer_ownership_tests.rs`)
- [ ] Owned by default for non-Copy types
- [ ] Borrowed when only read
- [ ] Mut borrowed when mutated
- [ ] Explicit `&T` respected
- [ ] Explicit `&mut T` respected
- [ ] Copy types passed by value
- [ ] Storage detection (push, insert, assign to field)
- [ ] Iterator variable tracking
- [ ] Generic parameter handling

#### 3.2 Type Inference Tests (`analyzer_type_tests.rs`)
- [ ] Literal type inference
- [ ] Variable type from assignment
- [ ] Return type from function
- [ ] Generic type instantiation
- [ ] Closure parameter inference

#### 3.3 Trait Analysis Tests (`analyzer_trait_tests.rs`)
- [ ] Trait method signature matching
- [ ] Default method implementations
- [ ] Trait bound checking
- [ ] Auto trait derivation

### 4. Code Generator Tests

#### 4.1 String Handling Tests (`codegen_string_tests.rs`)
- [ ] String literals to String (.to_string())
- [ ] String literals to &str (no conversion)
- [ ] &String parameters
- [ ] String concatenation with +
- [ ] format!() macro generation
- [ ] String methods (contains, replace, split, etc.)

#### 4.2 Method Call Tests (`codegen_method_tests.rs`)
- [ ] Instance methods (&self)
- [ ] Mutable methods (&mut self)
- [ ] Static methods (Self::new())
- [ ] Chained method calls
- [ ] Generic method calls
- [ ] Turbofish syntax

#### 4.3 Loop Tests (`codegen_loop_tests.rs`)
- [ ] For-in with .iter() inference
- [ ] For-in with .iter_mut() inference
- [ ] For-in over ranges
- [ ] While loops
- [ ] Loop variable mutation detection
- [ ] Iterator cloning for push

#### 4.4 Match Tests (`codegen_match_tests.rs`)
- [ ] Literal patterns
- [ ] Variable patterns
- [ ] Tuple patterns
- [ ] Struct patterns
- [ ] Enum patterns
- [ ] Guard conditions
- [ ] Or patterns (a | b)
- [ ] Wildcard pattern (_)
- [ ] Match arm type consistency

#### 4.5 Closure Tests (`codegen_closure_tests.rs`)
- [ ] Move closures (auto-inferred)
- [ ] Capture by reference
- [ ] Capture by value
- [ ] Async closures
- [ ] Closure return types

### 5. Auto-Derive Tests (`auto_derive_tests.rs`)
- [ ] @auto generates Clone
- [ ] @auto generates Debug
- [ ] @auto generates PartialEq when possible
- [ ] @auto generates Copy for simple types
- [ ] @auto generates Default when applicable
- [ ] Recursive type checking for Copy
- [ ] Skip PartialEq for non-comparable fields

### 6. Standard Library Tests (`stdlib_tests.rs`)
- [ ] std::fs operations
- [ ] std::http client
- [ ] std::json parsing/serialization
- [ ] std::testing assertions
- [ ] std::collections (Vec, HashMap, etc.)

### 7. FFI Tests (`ffi_tests.rs`)
- [ ] Extern function declarations
- [ ] Calling Rust functions
- [ ] Type mapping (Windjammer <-> Rust)
- [ ] Return value handling
- [ ] Error handling across FFI

### 8. Module System Tests (`module_tests.rs`)
- [ ] Single file compilation
- [ ] Multi-file projects
- [ ] use statements
- [ ] Visibility (pub/private)
- [ ] Circular dependency handling
- [ ] super:: paths
- [ ] crate:: paths

### 9. Error Handling Tests (`error_tests.rs`)
- [ ] Syntax error messages
- [ ] Type error messages
- [ ] Ownership error messages
- [ ] Error recovery
- [ ] Source location accuracy
- [ ] Suggestion generation

### 10. End-to-End Tests (`e2e_tests.rs`)
- [ ] Hello World
- [ ] Simple function
- [ ] Struct with methods
- [ ] Generic containers
- [ ] Trait implementations
- [ ] Game loop pattern
- [ ] Async operations

## Test Fixtures

Create test fixtures in `tests/fixtures/` for reusable test code:
- `fixtures/basic_types.wj` - Common type definitions
- `fixtures/trait_impls.wj` - Trait implementation examples
- `fixtures/generics.wj` - Generic type examples
- `fixtures/closures.wj` - Closure examples

## Running Tests

```bash
# Run all tests
cargo test --release

# Run specific test file
cargo test --release --test lexer_tests

# Run with output
cargo test --release -- --nocapture

# Run single test
cargo test --release test_name

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --release --out Html
```

## Coverage Goals

| Module | Current | Target |
|--------|---------|--------|
| Lexer | ~60% | 100% |
| Parser | ~70% | 100% |
| Analyzer | ~50% | 100% |
| Codegen | ~40% | 100% |
| Overall | ~55% | 100% |

## Timeline

1. **Phase 1**: Lexer & Parser tests (foundation)
2. **Phase 2**: Analyzer tests (ownership, types)
3. **Phase 3**: Codegen tests (string, methods, loops)
4. **Phase 4**: Integration & E2E tests
5. **Phase 5**: Edge cases & error handling

## Contributing

When adding new features:
1. Write failing tests first (TDD)
2. Implement the feature
3. Ensure all tests pass
4. Add documentation comments
5. Update this coverage plan

