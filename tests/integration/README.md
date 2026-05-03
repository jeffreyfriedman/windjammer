# Backend Integration Conformance Tests

## Purpose

Verify that Windjammer programs produce **identical output** across all runnable backends:

| Backend | Command | Notes |
|---------|---------|-------|
| **Rust** | `wj build --target rust` → `rustc` → run | Reference implementation |
| **Go** | `wj build --target go` → `go run` | Skipped if Go not installed |
| **JavaScript** | `wj build --target javascript` → `node` | Requires Node.js |
| **Interpreter** | Direct AST execution | Windjammerscript tree-walking |

**WGSL** is excluded: shader-only target, no `main()` or `println`.

## Test Cases

| File | Coverage |
|------|----------|
| `basic.wj` | Variables, functions, structs, enums, control flow |
| `ownership.wj` | Copy semantics, mutation, parameter inference |
| `patterns.wj` | Match, match guards, if let |
| `traits.wj` | Traits, implementations, generics (Rust + Interpreter only) |
| `strings.wj` | Literals, concatenation, interpolation |
| `collections.wj` | Vec push, len, indexing, iteration |

## Running Tests

```bash
# All integration tests
cargo test integration_backend

# Single test
cargo test test_integration_basic
```

## Philosophy

"No shortcuts, proper fixes with TDD" — these tests define the semantic contract that all backends must satisfy.
