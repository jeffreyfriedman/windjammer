# Changelog

All notable changes to Windjammer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Module aliases (`use X as Y`)
- Turbofish syntax (`Vec::<T>::new()`)
- Performance benchmarks vs Rust/Go
- Full Rust crate interop for stdlib modules (json, http, csv, etc.)

## [0.6.0] - 2025-10-05

### Added - Generics, User Modules & Idiomatic Rust 🚀
- **Basic Generics Support**:
  - Generic type parameters on functions: `fn identity<T>(x: T) -> T`
  - Generic type parameters on structs: `struct Box<T> { value: T }`
  - Generic type parameters on impl blocks: `impl<T> Box<T> { ... }`
  - Parameterized types: `Vec<T>`, `Option<T>`, `Result<T, E>`, custom types
  - Full AST support and Rust code generation
- **User-Defined Modules**:
  - Relative imports: `use ./utils`, `use ../shared/helpers`
  - Directory modules with `mod.wj` (similar to Rust's `mod.rs`)
  - `pub` keyword for module functions
  - Seamless integration with stdlib modules
- **Automatic Cargo.toml Dependency Management**:
  - Tracks stdlib module usage across all files
  - Auto-generates `[dependencies]` for required Rust crates
  - Creates `[[bin]]` section when `main.rs` exists
  - Supports application-style projects with lock files
- **Idiomatic Rust Type Generation**:
  - `&string` → `&str` (not `&String`) for better Rust interop
  - String literals and parameters now work seamlessly
  - Follows Rust best practices for string handling
- **Simplified Standard Library**:
  - `std/math` - Mathematical functions (✅ fully tested)
  - `std/strings` - String utilities (✅ fully tested)
  - `std/log` - Logging framework (✅ fully tested)
  - Deferred complex modules (json, http, csv) to post-v0.6.0

### Changed
- Updated `parse_type` to handle parameterized types
- Extended `FunctionDecl`, `StructDecl`, `ImplBlock` with `type_params`
- Added `Type::Generic` and `Type::Parameterized` variants
- Enhanced module path resolution for relative imports
- Refactored `ModuleCompiler` to track Cargo dependencies

### Fixed
- **Instance method calls** (`x.abs()`) vs **static calls** (`Type::method()`)
  - Correctly distinguishes based on identifier case and context
  - Fixed codegen bug where all method calls in modules used `::`
- String type handling for better Rust compatibility
- Module function visibility (`pub` prefix)

### Examples
- `examples/17_generics_test` - Basic generics demo
- `examples/18_stdlib_math_test` - std/math validation
- `examples/19_stdlib_strings_test` - std/strings validation
- `examples/20_stdlib_log_test` - std/log validation
- `examples/16_user_modules` - User-defined modules demo

### Documentation
- Updated `CHANGELOG.md` for all releases
- `docs/GENERICS_IMPLEMENTATION.md` - Implementation plan
- `docs/V060_PLAN.md` and `docs/V060_PROGRESS.md`

## [0.5.0] - 2025-10-04

### Added - Module System & Standard Library 🎉
- **Complete Module System**:
  - Module resolution from `std/` directory
  - Recursive dependency compilation
  - Automatic `pub mod` wrapping
  - Smart `::` vs `.` separator for Rust interop
  - Context-aware code generation with `is_module` flag
- **"Batteries Included" Standard Library** (11 modules, 910 lines):
  - `std/json` - JSON parsing/serialization (serde_json wrapper)
  - `std/csv` - CSV data processing
  - `std/http` - HTTP client (reqwest wrapper)
  - `std/fs` - File system operations ✅ **TESTED & WORKING**
  - `std/time` - Date/time operations (chrono wrapper)
  - `std/strings` - String manipulation utilities
  - `std/math` - Mathematical functions
  - `std/log` - Logging framework
  - `std/regex` - Regular expressions
  - `std/encoding` - Base64, hex, URL encoding
  - `std/crypto` - Cryptographic hashing
- **All stdlib modules written in Windjammer itself** (not compiler built-ins)
- **New Examples**:
  - `examples/10_module_test` - Module imports demo
  - `examples/11_fs_test` - File system operations (100% working)
  - `examples/12_simple_test` - Core language validation
  - `examples/13_stdlib_demo` - Multiple module usage
- **Comprehensive Documentation**:
  - `docs/MODULE_SYSTEM.md` - Complete 366-line guide
  - Updated README with "Batteries Included" section
  - 5 progress/status documents

### Fixed
- **CRITICAL**: Qualified path handling for stdlib modules
  - Windjammer paths (`std.fs.read`) now correctly convert to Rust (`std::fs::read`)
  - Smart separator detection: `::` for static calls, `.` for instance methods
  - Context-aware FieldAccess generation
- **CRITICAL**: Module function visibility (auto-add `pub` in module context)

### Changed
- Codegen now tracks module context with `is_module` flag
- Expression generation context-aware for paths vs field access
- MethodCall generation distinguishes static vs instance calls

## [0.4.0] - 2025-10-03

### Added
- **Implementation-Agnostic Abstractions**:
  - `@export` decorator replaces `@wasm_bindgen` for semantic external visibility
  - Compilation target system (`--target wasm|node|python|c`)
  - Implicit import injection based on decorators
  - Multi-layered target detection system
- **Standard Library Foundation**:
  - Initial stdlib module specifications (json, http, fs, time, strings, math, log)
  - Design for "batteries included" approach
- **WASM Examples**:
  - `wasm_hello` - Simple WASM functions (greet, add, Counter)
  - `wasm_game` - Conway's Game of Life running at 60 FPS in browser
- Character literals with escape sequences (`'a'`, `'\n'`, `'\t'`, `'\''`, `'\\'`, `'\0'`)
- Struct field decorators for CLI args, serialization, validation
- Decorator support for `impl` blocks
- Comprehensive test suite (57 tests total)
- 5 working basic example projects

### Fixed
- **CRITICAL**: Binary operator precedence bug
- **CRITICAL**: Glob imports for `use` statements
- **CRITICAL**: Impl block decorators parsing and generation
- **CRITICAL**: Functions in `#[wasm_bindgen]` impl blocks now `pub`
- **MAJOR**: Match expression parsing (struct literal disambiguation)

### Changed
- Removed `@wasm_bindgen` from examples, replaced with `@export`
- Compiler now maps decorators based on compilation target

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

- **v0.5** - Module system & "batteries included" standard library (11 modules)
- **v0.4** - Implementation-agnostic abstractions, @export decorator, WASM examples
- **v0.3** - Ergonomic improvements (ternary, smart derive)
- **v0.2** - Modern features (interpolation, pipe, patterns)
- **v0.1** - Core language and compiler

