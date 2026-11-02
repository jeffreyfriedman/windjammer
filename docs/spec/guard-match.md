# Guard-only match

A `match` without a scrutinee, Elixir `cond` style ([ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 7). Lowered by the parser to a `match` with guards, so every backend sees an existing construct.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/guard-match/`.

## Guard-only match

**SPEC-MATCH-001** `match { c1 => v1, c2 => v2, _ => v3 }` (no scrutinee) evaluates the conditions in order and yields the value of the first arm whose condition is true, else the `_` arm. It is `match true { _ if c1 => v1, _ if c2 => v2, _ => v3 }`.

<!-- example: tests/spec/guard-match/SPEC-MATCH-001.wj -->
```windjammer
fn sign(x: int) -> string {
    match {
        x < 0 => "negative",
        x == 0 => "zero",
        _ => "positive",
    }
}

fn main() {
    println("${sign(-3)} ${sign(0)} ${sign(8)}")
}
```
Output:
```text
negative zero positive
```

**SPEC-MATCH-002** Arms may be blocks or statements, as in any `match`; a guard-only match is also a statement.

<!-- example: tests/spec/guard-match/SPEC-MATCH-002.wj -->
```windjammer
fn main() {
    let mut n = 0
    for x in 0..10 {
        match {
            x % 3 == 0 => {
                n = n + 1
            }
            _ => {}
        }
    }
    println("${n}")
}
```
Output:
```text
4
```

**SPEC-MATCH-003** A guard-only match must end with a `_ =>` arm (the conditions are not known to be exhaustive).

<!-- example: tests/spec/guard-match/SPEC-MATCH-003.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn main() {
    let x = 3
    let s = match {
        x > 0 => "pos",
        x <= 0 => "non-pos",
    }
}
```
Error contains: `needs a final `_ => ..` arm`

**SPEC-MATCH-004** An arm after the `_` arm is unreachable and an error.

<!-- example: tests/spec/guard-match/SPEC-MATCH-004.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn main() {
    let x = 3
    let s = match {
        _ => "any",
        x > 0 => "pos",
    }
}
```
Error contains: `must be the last arm`
