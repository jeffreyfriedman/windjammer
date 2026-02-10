# Windjammer Conformance Test Suite

## Purpose

These tests define **what correct Windjammer behavior looks like**, independent of
any compilation backend. Every test must produce identical output whether compiled
to Rust, Go, or any future target.

The conformance suite is the **source of truth** for Windjammer's semantic contract.
If a backend produces different output from another backend on any conformance test,
that backend has a bug.

## Philosophy

> The analyzer enforces safety. The backend translates. The conformance tests prove
> they agree.

These tests deliberately focus on **observable behavior** (stdout output), not
implementation details (generated code patterns). A test doesn't care whether a
parameter is passed as `&Vec<i32>` (Rust) or `[]int` (Go) — it cares that the
function produces the right result.

## Test Structure

```
conformance/
├── README.md                       # This file
├── run_conformance_tests.sh        # Test runner script
├── values/
│   ├── copy_semantics.wj           # Copy types are independent
│   ├── mutation_semantics.wj       # Mutation rules and parameter passing
│   └── clone_semantics.wj          # Explicit cloning creates independent copies
├── types/
│   ├── enums_and_matching.wj       # ADTs, pattern matching, exhaustiveness
│   ├── structs_and_methods.wj      # Structs, impl blocks, constructors, methods
│   └── traits_and_generics.wj      # Traits, generics, default implementations
├── control_flow/
│   └── control_flow.wj             # if/else, while, for, loop, break, continue
├── error_handling/
│   └── result_and_option.wj        # Option, Result-like patterns, match handling
├── stdlib/
│   ├── vec_operations.wj           # Vec creation, push, pop, iteration
│   ├── string_operations.wj        # String literals, concat, interpolation, methods
│   ├── hashmap_operations.wj       # HashMap insert, get, remove, contains
│   └── closures_and_iteration.wj   # Closures, capture, higher-order patterns
└── integrated_game_logic.wj        # Realistic game scenario combining all features
```

## Test Format

Every conformance test is a valid `.wj` file with:

1. **A header comment** documenting the semantic contract being tested
2. **Expected output** listed in comments (prefixed with `// `)
3. **A `main()` function** that runs all test cases
4. **A `PASSED` marker** printed at the end if all internal checks pass
5. **Tagged output** with `[test_name]` prefixes for clear identification

Example:

```windjammer
// Conformance Test: Copy Semantics
//
// EXPECTED OUTPUT:
// [copy_int] a=42, b=42
// [copy_int] after b=99: a=42, b=99
// [copy_all] PASSED

fn main() {
    let a = 42
    let b = a
    println("[copy_int] a=${a}, b=${b}")

    let mut b = a
    b = 99
    println("[copy_int] after b=99: a=${a}, b=${b}")
    println("[copy_all] PASSED")
}
```

## Running Tests

### Current (Rust backend only)

```bash
# Run all conformance tests
./tests/conformance/run_conformance_tests.sh

# Run a specific test
./tests/conformance/run_conformance_tests.sh values/copy_semantics.wj
```

### Future (Multi-backend)

When the Go backend is added, the runner will:

```bash
# Compile and run on ALL backends, compare output
./tests/conformance/run_conformance_tests.sh --all-backends

# Compare specific backends
./tests/conformance/run_conformance_tests.sh --backends rust,go
```

The runner will:
1. Compile each `.wj` file to Rust and Go
2. Build both outputs
3. Run both binaries
4. Assert stdout is **identical** (byte-for-byte)
5. Report any divergences

## Semantic Contract Coverage

| Category | Tests | What It Verifies |
|----------|-------|------------------|
| **Values** | 3 | Copy types, mutation, clone independence |
| **Types** | 3 | Enums/ADTs, structs/methods, traits/generics |
| **Control Flow** | 1 | if/else, while, for, loop, break, continue, short-circuit |
| **Error Handling** | 1 | Option, Result-like patterns |
| **Stdlib** | 4 | Vec, String, HashMap, closures/iteration |
| **Integrated** | 1 | Realistic game logic combining all features |
| **Total** | **13** | |

## Adding New Tests

When adding a conformance test:

1. **Create the `.wj` file** in the appropriate subdirectory
2. **Include expected output** in header comments
3. **Tag all output** with `[test_name]` prefixes
4. **End with `PASSED`** so the runner can verify
5. **Focus on observable behavior**, not implementation details
6. **Test one semantic concept** per file (or use `integrated/` for combined)

### Guidelines

- **DO**: Test that values are correct, mutations are visible, patterns match
- **DON'T**: Test that generated code contains specific Rust/Go syntax
- **DO**: Use `println` for all output (machine-comparable)
- **DON'T**: Use `assert` that would crash — prefer printing and checking output
- **DO**: Include edge cases (empty collections, zero values, boundary conditions)
- **DON'T**: Rely on hash iteration order (HashMap order is non-deterministic)

## Relation to Go Backend Plan

This conformance suite is a **prerequisite** for the Go backend (see
`docs/GO_BACKEND_PLAN.md`). The readiness criteria state:

> A conformance test suite exists (backend-independent tests)

These tests define the contract that any backend must satisfy. When the Go backend
is implemented, these same tests will validate it produces identical behavior to
the Rust backend.
