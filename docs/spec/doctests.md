# Doctests

Code examples in doc comments are tests ([ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 6). One extractor serves both `wj test` and the documentation checker (`src/doctest/`).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/doctests/`.

## Doctests

**SPEC-DOC-001** A ```` ```wj ```` (or ```` ```windjammer ````) fence inside `///` or `//!` doc comments of a `.wj` file is a doctest. `wj test` runs every doctest (`wj test --doc` runs only doctests). The fence runs with its module in scope: the program is the module source, with its own `fn main` renamed away, followed by the fence (wrapped in `fn main` when it is a statement fragment). Verified by `tests/doctest_cli_test.rs`.

<!-- example: tests/spec/doctests/SPEC-DOC-001.wj -->
```windjammer
/// Doubles a number.
///
/// ```wj
/// println("${double(21)}")
/// // expect: 42
/// ```
fn double(x: int) -> int {
    x * 2
}

fn main() {
    println("${double(1)}")
}
```

**SPEC-DOC-002** `// expect: text` lines in the fence are the expected standard output, line by line and in order; a doctest without them must run to completion without panicking. The module's own comments never count as expectations.

<!-- example: tests/spec/doctests/SPEC-DOC-002.wj -->
```windjammer
/// Doubles a number.
///
/// ```wj
/// println("${double(21)}")
/// // expect: 42
/// ```
fn double(x: int) -> int {
    x * 2
}

fn main() {
    println("${double(1)}")
}
```

**SPEC-DOC-003** Doctests run in the interpreter by default; `wj test --compiled` also builds and runs each one through the Rust backend, and both outputs must equal the expectation. Annotations are those of the documentation checker: `ignore` skips, `should_fail` / `should_fail=CODE` must fail to parse or analyze (with that code), `check` must also pass the analyzer.

<!-- example: tests/spec/doctests/SPEC-DOC-003.wj -->
```windjammer
/// Not runnable on purpose.
///
/// ```wj,should_fail
/// let x: int = "text" +
/// ```
fn noop() {}

fn main() {
    noop()
}
```
