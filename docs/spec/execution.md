# Execution model and cross-backend guarantees

What every conformant backend must do identically, graded by the tiers of ADR-0007 (status: Proposed).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/execution/`.

**Backends today (evidence: docs/backends/CONFORMANCE.md, tests/conformance_programs/SUPPORT.toml).** Rust (Tier 1), Go/JavaScript/TypeScript (portable emitters), WebAssembly (via the Rust generator), the interpreter (`wj run --interpret`, Tier 3). The shared corpus has 98+ programs with independent oracles; `SUPPORT.toml` lists the known-failing pairs: 27 Rust entries (almost all the `rust-int-literal-width-inference` known issue) and the Go/JS/TS gaps for `{:+}`/`{:#}` flags and pretty debug. Everything in this spec is checked here against the **interpreter only**; the interpreter is explicitly the weakest voice, so each example's expectation is derived by hand and cross-checked against Python/Rust semantics, never copied from the interpreter's output.

| Guarantee | Rule |
|---|---|
| stdout + exit status are the observable behaviour | EXEC-001, EXEC-002 |
| 64-bit checked `int` everywhere | EXEC-004, NUM-001..003 |
| `HashMap` order unspecified | EXEC-005 |
| float text | EXEC-007 |
| recursion depth >= 10,000 | EXEC-006 |

## Observable behavior

**SPEC-EXEC-001** The observable behavior of a program is the sequence of bytes written to standard output and its exit status. Standard error text, timing and memory layout are not part of the conformance contract.

<!-- example: tests/spec/execution/SPEC-EXEC-001.wj -->
```windjammer
fn main() {
    println("out1")
    eprintln("err")
    println("out2")
}
```
Output:
```text
out1
out2
```

## Exit status

**SPEC-EXEC-002** Returning from `main` exits 0; an unhandled panic exits 101 after flushing everything already written to standard output; `std::process::exit(n)` exits `n`.

<!-- example: tests/spec/execution/SPEC-EXEC-002.wj -->
```windjammer
use std::process
fn main() {
    println("bye")
    process::exit(3)
}
```
Output:
```text
bye
```
Then panics, exit status 3.

## Panic output ordering

**SPEC-EXEC-003** Output written before a panic is delivered in order; nothing after the panicking statement runs.

<!-- example: tests/spec/execution/SPEC-EXEC-003.wj -->
```windjammer
fn main() {
    println("1")
    let v = [1]
    let i = 5
    println("2")
    println("${v[i]}")
    println("3")
}
```
Output:
```text
1
2
```
Then panics (message contains `out of bounds`), exit status 101.

## Integer semantics are backend independent

**SPEC-EXEC-004** `int` is 64-bit two's complement with checked arithmetic on every backend, including JavaScript (which must not use IEEE doubles for integers above 2^53); fixed-width types obey their ranges.

<!-- example: tests/spec/execution/SPEC-EXEC-004.wj -->
```windjammer
fn main() {
    let a = 9007199254740993
    let b = a + 2
    println("${a} ${b}")
    let m = 3037000499
    println("${m * m - 1 > 0}")
}
```
Output:
```text
9007199254740993 9007199254740995
true
```

## Unordered collections

**SPEC-EXEC-005** The iteration order of `HashMap` and `HashSet` is unspecified and may differ between backends and runs; portable programs sort before observing order.

<!-- example: tests/spec/execution/SPEC-EXEC-005.wj -->
```windjammer
use std::collections::HashMap
fn main() {
    let mut m = HashMap::new()
    m.insert(3, "c")
    m.insert(1, "a")
    m.insert(2, "b")
    let mut ks = m.keys().collect::<Vec<&int>>()
    ks.sort()
    let mut out = ""
    for k in ks { out = out + m.get(k).unwrap() }
    println(out)
}
```
Output:
```text
abc
```

## Deep recursion

**SPEC-EXEC-006** [OPEN Q-023] Programs may recurse at least 10,000 frames deep (plus `main`) on every backend without stack overflow. The interpreter caps the call stack at exactly 10,000 frames including `main`, so this example overflows it.

<!-- example: tests/spec/execution/SPEC-EXEC-006.wj -->
<!-- ignore: known divergence Q-023; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn sum_to(n: int) -> int {
    if n == 0 { 0 } else { n + sum_to(n - 1) }
}
fn main() {
    println("${sum_to(10000)}")
}
```
Output:
```text
50005000
```
> **Known divergence:** the implementation does not do this yet; see [Q-023](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Float text is backend independent

**SPEC-EXEC-007** Float to text follows EXEC rule NUM-013 on every backend: shortest round-trip digits, no exponent, `inf`/`-inf`/`NaN`, and precision rounding half to even. [Tier note: Go and JavaScript emit exponents for very large or small magnitudes; see OPEN Q-018.]

<!-- example: tests/spec/execution/SPEC-EXEC-007.wj -->
```windjammer
fn main() {
    println("${0.1} ${100.0 / 3.0} ${1.0 / 3.0:.4}")
}
```
Output:
```text
0.1 33.333333333333336 0.3333
```

## Target tiers

**SPEC-EXEC-008** Backends are graded by ADR-0007: Tier 1 Rust (every rule applies and gates releases), Tier 2 WebAssembly and JavaScript/TypeScript (the documented conformance subset must pass), Tier 3 Go, WGSL and the interpreter (experimental, no stability promise). A feature is promoted only when the shared conformance suite passes on it. (untestable: policy statement; enforced by tests/backend_conformance_test.rs and tests/conformance_programs/SUPPORT.toml)

## Single-threaded determinism

**SPEC-EXEC-009** A program without concurrency primitives, randomness, clock reads, environment or file/network input is deterministic: the same inputs give the same output on every conformant backend.

<!-- example: tests/spec/execution/SPEC-EXEC-009.wj -->
```windjammer
fn lcg(s: int) -> int { (s * 1103515245 + 12345) % 2147483648 }
fn main() {
    let mut s = 1
    let mut i = 0
    while i < 3 {
        s = lcg(s)
        println("${s}")
        i += 1
    }
}
```
Output:
```text
1103527590
377401575
662824084
```

