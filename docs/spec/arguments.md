# Named arguments and defaults

Named arguments and parameter defaults ([ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 3). The parser rewrites every call to a callee declared in the same file into a positional call with the defaults filled in, so all backends see ordinary calls. Arguments are evaluated in parameter order.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/arguments/`.

## Named arguments and defaults

**SPEC-ARGS-001** A parameter may declare a default, `name: Type = expr`. At a call, positional arguments come first, then named ones (`name: value`) in any order; parameters not given take their defaults.

<!-- example: tests/spec/arguments/SPEC-ARGS-001.wj -->
```windjammer
fn connect(host: string, timeout: int = 30, retries: int = 3) -> string {
    "${host}:${timeout}:${retries}"
}

fn main() {
    println(connect("db"))
    println(connect("db", retries: 5))
    println(connect(retries: 9, host: "h"))
}
```
Output:
```text
db:30:3
db:30:5
h:30:9
```

**SPEC-ARGS-002** A default is evaluated at the call site, on every call that uses it (never shared between calls).

<!-- example: tests/spec/arguments/SPEC-ARGS-002.wj -->
```windjammer
fn tick() -> int {
    println("tick")
    1
}

fn step(n: int = tick()) -> int {
    n
}

fn main() {
    let a = step()
    let b = step(5)
    let c = step()
    println("${a + b + c}")
}
```
Output:
```text
tick
tick
7
```

**SPEC-ARGS-003** Methods and associated functions take named arguments and defaults the same way (resolved when the method name is defined by one `impl` in the file; otherwise call positionally).

<!-- example: tests/spec/arguments/SPEC-ARGS-003.wj -->
```windjammer
struct Counter {
    n: int,
}

impl Counter {
    fn bump(self, by: int = 1) -> int {
        self.n + by
    }
}

fn main() {
    let c = Counter { n: 10 }
    println("${c.bump()} ${c.bump(by: 5)}")
}
```
Output:
```text
11 15
```

## Errors

**SPEC-ARGS-004** An unknown parameter name is an error that lists the parameters.

<!-- example: tests/spec/arguments/SPEC-ARGS-004.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn connect(host: string, timeout: int = 30, retries: int = 3) -> string {
    "${host}:${timeout}:${retries}"
}

fn main() {
    println(connect("db", retry: 1))
}
```
Error contains: `has no parameter `retry``

**SPEC-ARGS-005** A parameter given twice is an error.

<!-- example: tests/spec/arguments/SPEC-ARGS-005.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn connect(host: string, timeout: int = 30, retries: int = 3) -> string {
    "${host}:${timeout}:${retries}"
}

fn main() {
    println(connect("db", timeout: 1, timeout: 2))
}
```
Error contains: `is given twice`

**SPEC-ARGS-006** A positional argument after a named one is an error.

<!-- example: tests/spec/arguments/SPEC-ARGS-006.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn connect(host: string, timeout: int = 30, retries: int = 3) -> string {
    "${host}:${timeout}:${retries}"
}

fn main() {
    println(connect(timeout: 1, "db"))
}
```
Error contains: `positional argument after a named one`

**SPEC-ARGS-007** A required parameter (no default) that is not given is an error.

<!-- example: tests/spec/arguments/SPEC-ARGS-007.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn connect(host: string, timeout: int = 30, retries: int = 3) -> string {
    "${host}:${timeout}:${retries}"
}

fn main() {
    println(connect(timeout: 1))
}
```
Error contains: `missing argument `host``

**SPEC-ARGS-008** Parameters with defaults must come after those without.

<!-- example: tests/spec/arguments/SPEC-ARGS-008.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn f(a: int = 1, b: int) -> int {
    a + b
}

fn main() {
    println("${f(1, 2)}")
}
```
Error contains: `put parameters with defaults last`

**SPEC-ARGS-009** A default may not refer to another parameter (it is evaluated before the call, in the caller).

<!-- example: tests/spec/arguments/SPEC-ARGS-009.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn pad(s: string, width: int = s.len()) -> int {
    width
}

fn main() {
    println("${pad("ab")}")
}
```
Error contains: `cannot refer to parameter `s``
