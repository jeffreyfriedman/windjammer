# Changelog

All notable changes to Windjammer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **WASM Examples**:
  - `wasm_hello` - Simple WASM functions (greet, add, Counter)
  - `wasm_game` - Conway's Game of Life running at 60 FPS in browser
- Character literals with escape sequences (`'a'`, `'\n'`, `'\t'`, `'\''`, `'\\'`, `'\0'`)
- Struct field decorators for CLI args, serialization, validation
- Decorator support for `impl` blocks (e.g., `@wasm_bindgen`)
- Comprehensive test suite (57 tests total):
  - 16 lexer tests (all token types)
  - 9 compiler integration tests
  - 32 feature test framework
- 5 working example projects (385 lines total):
  - 01_basics - Functions, loops, ternary, string interpolation
  - 02_structs - Structs, impl blocks, methods
  - 03_enums - Enums, pattern matching, OR patterns
  - 04_traits - Trait definitions and implementations
  - 05_modern - Pipe operator, labeled args, ranges, characters
- Comprehensive documentation:
  - SESSION_COMPLETE.md - Session summary
  - COMPREHENSIVE_STATUS.md - Detailed status
  - WASM_FIXES.md - Detailed bug report from WASM development
  - Updated GUIDE.md with character literals and field decorators

### Fixed
- **CRITICAL**: Binary operator precedence bug (e.g., `a + b % c` now generates correct parentheses)
- **CRITICAL**: Glob imports for `use` statements (now generates `use crate::*;` for WASM compatibility)
- **CRITICAL**: Impl block decorators weren't being parsed or generated
- **CRITICAL**: Functions in `#[wasm_bindgen]` impl blocks now correctly marked as `pub`
- **MAJOR**: Match expression parsing (confused `match x {}` with struct literals)
  - Now supports `match x {}`, `match &x {}`, `match *x {}`, etc.
  - Added unary operator support in match values
  - Fixed match as return expression
- Parser now correctly handles match expressions in all contexts
- Match expressions work with references and dereferences

### Changed
- Updated `parse_match()` to use `parse_match_value()` instead of `parse_expression()`
- Enhanced `parse_match_value()` with unary operator support (&, *, -, !)
- Improved parser disambiguation for match vs struct literals

## [0.3.0] - 2025-10-03

### Added
- Ternary operator for concise conditional expressions
- Intelligent `@auto` derive that infers traits based on field types
- Test fixtures for all major features
- Comprehensive documentation

### Changed
- `@auto` decorator now supports zero arguments for smart inference
- Updated README with accurate language description

## [0.2.0] - 2025-10-02

### Added
- String interpolation with `${expr}` syntax
- Pipe operator (`|>`) for data transformations
- Labeled/named function arguments
- Pattern matching in function parameters
- Explicit `@auto` derive decorator
- Trait system (definitions and implementations)
- Automatic reference insertion at call sites
- Tuple types and patterns

### Fixed
- Trailing semicolons in return expressions
- String interpolation bug with println! macro
- Parser disambiguation for `?` operator

## [0.1.0] - 2025-10-01

### Added
- Core compiler pipeline (lexer, parser, analyzer, codegen)
- Basic language features:
  - Functions (regular and async)
  - Structs and enums
  - Impl blocks with methods
  - Pattern matching with guards
  - For/while/loop constructs
  - Closures and ranges
  - Go-style concurrency (`go` keyword)
  - Go-style channels with `<-` operator
- Automatic ownership inference
- CLI with `build` and `check` commands
- Examples: hello_world, http_server, wasm_game, cli_tool

### Core Philosophy
- 80/20 Rule: 80% of Rust's power with 20% of complexity
- Inspired by Go, Ruby, Elixir, Python, and Rust
- Transpiles to idiomatic Rust code

---

## Version History Summary

- **v0.3** - Ergonomic improvements (ternary, smart derive)
- **v0.2** - Modern features (interpolation, pipe, patterns)
- **v0.1** - Core language and compiler

