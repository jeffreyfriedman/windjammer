# Changelog

All notable changes to Windjammer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Ternary operator (`condition ? true_val : false_val`)
- Smart `@auto` derive with zero-config trait inference
- Dual MIT/Apache-2.0 licensing
- Comprehensive test suite (8 integration tests)
- CONTRIBUTING.md for contributors

### Changed
- Updated project description to reflect multi-language inspiration
- Reorganized documentation into `docs/` folder

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

