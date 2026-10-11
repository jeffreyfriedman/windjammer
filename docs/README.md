# Windjammer Documentation

Welcome to the Windjammer documentation! This folder contains all core documentation for the Windjammer programming language and compiler.

## Documentation Structure

### Core Documentation (This Folder)

**Language & Architecture:**
- [`ARCHITECTURE.md`](ARCHITECTURE.md) - Compiler architecture overview
- [`COMPARISON.md`](COMPARISON.md) - Language comparison (vs Rust, Go, Python)
- [`MODULE_SYSTEM.md`](MODULE_SYSTEM.md) - Module system design
- [`INDEX.md`](INDEX.md) - Documentation index

**Design & Implementation:**
- [`BEST_PRACTICES.md`](BEST_PRACTICES.md) - Coding best practices
- [`CONCURRENCY_ARCHITECTURE.md`](CONCURRENCY_ARCHITECTURE.md) - Concurrency model
- [`SALSA_ARCHITECTURE.md`](SALSA_ARCHITECTURE.md) - Incremental compilation
- [`STDLIB_ARCHITECTURE.md`](STDLIB_ARCHITECTURE.md) - Standard library design

**Multi-Backend:**
- [`CROSS_PLATFORM_VISION.md`](CROSS_PLATFORM_VISION.md) - Cross-platform strategy
- [`EXPORT_TARGETS.md`](EXPORT_TARGETS.md) - Compilation targets (Rust, Go, JS)
- [`MULTI_LANGUAGE_OPTIMIZATION.md`](MULTI_LANGUAGE_OPTIMIZATION.md) - Multi-backend optimization
- [`MULTI_LANGUAGE_SDK_ARCHITECTURE.md`](MULTI_LANGUAGE_SDK_ARCHITECTURE.md) - SDK design
- [`WEB_EXPORT_STRATEGY.md`](WEB_EXPORT_STRATEGY.md) - Web/WASM export

**Compiler Internals:**
- [`COMPILER_OPTIMIZATIONS.md`](COMPILER_OPTIMIZATIONS.md) - Optimization strategies
- [`OPTIMIZATION_ARCHITECTURE.md`](OPTIMIZATION_ARCHITECTURE.md) - Optimization pipeline

**Developer Resources:**
- [`API_REFERENCE.md`](API_REFERENCE.md) - API documentation
- [`COOKBOOK.md`](COOKBOOK.md) - Code examples and recipes
- [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md) - Testing approach (TDD + Dogfooding)
- [`USER_LIBRARY_TEST_GAPS.md`](USER_LIBRARY_TEST_GAPS.md) - Why dogfood crates still use Rust `cargo test` vs `wj test`
- [`TOOLING_VISION.md`](TOOLING_VISION.md) - Tooling ecosystem

**Project Management:**
- [`PUBLISHING.md`](PUBLISHING.md) - Publishing and release process
- [`VERSIONING.md`](VERSIONING.md) - Versioning and release policy

### Subdirectories

#### [`proposals/rfcs/`](proposals/rfcs/)
Request for Comments (RFCs) for major features:
- **Security**: WJ-SEC-01 (Effect Capabilities), WJ-SEC-02 (Taint Tracking), WJ-SEC-03 (Capability Lock File), WJ-SEC-04 (Capability Profiles)
- **Language**: WJ-LANG-01 (Shader Language)
- **Syntax**: WJ-SYN-01 (Pipe Operator), WJ-SYN-02 (Syntax Improvements)
- **Implementation**: WJ-IMPL-01 (Compiler Refactoring), WJ-IMPL-02 (FFI Generation)
- **Performance**: WJ-PERF-01 (Economic Efficiency)
- **Concurrency / packages / maps / strings**: WJ-CONC-01, WJ-PKG-01, WJ-LANG-03, WJ-LANG-04

See [`proposals/rfcs/README.md`](proposals/rfcs/README.md) for full RFC index.

#### [`design/`](design/)
Language design documents:
- Traits, async execution (superseded), multi-target codegen, error mapping, etc.

#### [`language-guide/`](language-guide/)
User-facing language documentation and tutorials.

#### [`tutorials/`](tutorials/)
Step-by-step tutorials for learning Windjammer.

#### [`adr/`](adr/README.md)
Architecture decision records. All are `Proposed` until the project owner accepts them.

#### [`archive/`](archive/README.md)
Superseded documents kept for history.

#### [`known-gaps.md`](known-gaps.md)
Ranked gaps between the docs/RFCs and the code.

## Quick Navigation

**Getting Started:**
- New to Windjammer? Start with [`language-guide/`](language-guide/)
- Want code examples? See [`COOKBOOK.md`](COOKBOOK.md)
- Understanding the compiler? Read [`ARCHITECTURE.md`](ARCHITECTURE.md)

**Contributing:**
- Read [`BEST_PRACTICES.md`](BEST_PRACTICES.md)
- Follow [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md) (TDD + Dogfooding)
- Check [`proposals/rfcs/`](proposals/rfcs/) for upcoming features

**Reference:**
- API docs: [`API_REFERENCE.md`](API_REFERENCE.md)
- Language comparison: [`COMPARISON.md`](COMPARISON.md)
- Multi-backend: [`CROSS_PLATFORM_VISION.md`](CROSS_PLATFORM_VISION.md)

## Windjammer Philosophy

Windjammer is built on these core principles:
1. **Compiler Does the Hard Work, Not the Developer** - 80% of Rust's power with 20% of its complexity
2. **Inference When It Doesn't Matter, Explicit When It Does** - Automatic ownership; whether local mutability is inferred or explicit is an open decision ([ADR-0002](adr/0002-local-mutability.md))
3. **No Workarounds, Only Proper Fixes** - Long-term robustness over short-term hacks
4. **Backend-Agnostic by Design** - Compiles to Rust, Go, JavaScript, and more

See [`.cursor/rules/`](../.cursor/rules/) for detailed development rules.

## Documentation Maintenance

**Adding new docs:**
- Place in appropriate subdirectory (`design/`, `proposals/rfcs/`, etc.)
- Use clear, descriptive filenames
- Update this README if adding major new categories

**Archiving old docs:**
- Superseded documents move to [`archive/`](archive/README.md) with a one-line reason; session logs and status snapshots are not kept
- Keep only current, relevant documentation in root

**Last cleaned:** 2026-10-10

---

**Questions?** See [`INDEX.md`](INDEX.md) for full documentation index.
