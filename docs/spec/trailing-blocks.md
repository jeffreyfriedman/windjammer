# Trailing block arguments

Trailing block arguments ([ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 4). The parser appends the block as an ordinary closure argument.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/trailing-blocks/`.

## Trailing blocks

**SPEC-BLOCK-001** A call `f(args)` followed on the same line by `{ body }` passes the block as one more positional argument, a closure without parameters.

<!-- example: tests/spec/trailing-blocks/SPEC-BLOCK-001.wj -->
```windjammer
fn run_with(label: string, body: fn() -> int) -> int {
    println("run ${label}")
    body()
}

fn main() {
    let r = run_with("job") { 40 + 2 }
    println("${r}")
}
```
Output:
```text
run job
42
```

**SPEC-BLOCK-002** `{ a, b -> body }` declares the closure parameters (optionally typed: `{ n: int -> .. }`).

<!-- example: tests/spec/trailing-blocks/SPEC-BLOCK-002.wj -->
```windjammer
fn apply_to(x: int, f: fn(int) -> int) -> int {
    f(x)
}

fn main() {
    let xs: Vec<int> = [1, 2, 3]
    let ys: Vec<int> = xs.map() { v -> v * 10 }.collect()
    println("${ys[2]} ${apply_to(3) { n: int -> n * 2 }}")
}
```
Output:
```text
30 6
```

**SPEC-BLOCK-003** In the header of `if`, `while`, `for` and `match`, a `{` after a call opens the body, never a trailing block.

<!-- example: tests/spec/trailing-blocks/SPEC-BLOCK-003.wj -->
```windjammer
fn big(x: int) -> bool {
    x > 2
}

fn main() {
    if big(3) {
        println("yes")
    }
    let mut i = 0
    while big(5 - i) {
        i = i + 1
    }
    println("${i}")
}
```
Output:
```text
yes
3
```

**SPEC-BLOCK-004** A call takes at most one trailing block; pass further callbacks as named arguments.

<!-- example: tests/spec/trailing-blocks/SPEC-BLOCK-004.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn both(a: fn() -> int, b: fn() -> int) -> int {
    a() + b()
}

fn main() {
    println("${both() { 1 } { 2 }}")
}
```
Error contains: `only one trailing block per call`
