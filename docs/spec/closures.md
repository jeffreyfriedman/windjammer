# Shorthand closures

`_` shorthand closures ([ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 2). The parser turns each one into an ordinary closure with parameters `_it0`, `_it1`, ...; types and ownership are those of the equivalent explicit closure.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/closures/`.

## Shorthand closures

**SPEC-CLOS-001** Inside a call argument, an expression containing `_` (other than the argument being exactly `_`) is a closure with one parameter per `_`: `xs.map(_ * 2)` is `xs.map(|x| x * 2)`, `users.filter(_.active)` is `users.filter(|u| u.active)`.

<!-- example: tests/spec/closures/SPEC-CLOS-001.wj -->
```windjammer
struct User {
    name: string,
    active: bool,
}

fn main() {
    let xs = [1, 2, 3]
    let doubled: Vec<int> = xs.map(_ * 2).collect()
    println("${doubled[0]} ${doubled[2]}")
    let users = [User { name: "ann", active: true }, User { name: "bob", active: false }]
    let names: Vec<string> = users.filter(_.active).map(_.name).collect()
    println(names.join(","))
}
```
Output:
```text
2 6
ann
```

**SPEC-CLOS-002** Each `_` is a new parameter, left to right: `xs.fold(0, _ + _)` is `xs.fold(0, |a, b| a + b)`.

<!-- example: tests/spec/closures/SPEC-CLOS-002.wj -->
```windjammer
fn main() {
    let xs = [1, 2, 3, 4]
    println("${xs.fold(0, _ + _)}")
    let z = [1, 2].zip_with([10, 20], _ * _)
    println("${z[0]} ${z[1]}")
}
```
Output:
```text
10
10 40
```

**SPEC-CLOS-003** The closure is the smallest enclosing call argument containing the `_`: `f(g(_ + 1))` is `f(g(|x| x + 1))`.

<!-- example: tests/spec/closures/SPEC-CLOS-003.wj -->
```windjammer
fn apply(f: fn(int) -> int, x: int) -> int {
    f(x)
}

fn twice(x: int) -> int {
    x * 2
}

fn main() {
    println("${twice(apply(_ + 1, 4))}")
}
```
Output:
```text
10
```

**SPEC-CLOS-004** In a `|>` stage a bare `_` argument is the pipe placeholder, and an argument that merely contains `_` is a shorthand closure: `5 |> apply(_ * 3, _)` is `apply(|x| x * 3, 5)`.

<!-- example: tests/spec/closures/SPEC-CLOS-004.wj -->
```windjammer
fn apply(f: fn(int) -> int, x: int) -> int {
    f(x)
}

fn main() {
    println("${5 |> apply(_ * 3, _)}")
    let xs = [1, 2]
    let z = xs |> .zip_with([3, 4], _ + _)
    println("${z[0]} ${z[1]}")
}
```
Output:
```text
15
4 6
```

## Misuse

**SPEC-CLOS-005** A bare `_` that is not a pipe placeholder is an error ("`_` alone is not a function"): write the function name or `|x| x`.

<!-- example: tests/spec/closures/SPEC-CLOS-005.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn id(x: int) -> int {
    x
}

fn main() {
    println("${id(_)}")
}
```
Error contains: ``_` alone is not a function`

**SPEC-CLOS-006** `_` outside any call argument is an error.

<!-- example: tests/spec/closures/SPEC-CLOS-006.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn main() {
    let f = _ + 1
}
```
Error contains: `only valid inside a call argument`

**SPEC-CLOS-007** `_` inside an explicit closure of the same argument is an error ("cannot be used inside an explicit closure"); name the parameter instead.

<!-- example: tests/spec/closures/SPEC-CLOS-007.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn main() {
    let xs = [1, 2]
    let ys: Vec<int> = xs.map(|x| x + _).collect()
}
```
Error contains: `inside an explicit closure`

**SPEC-CLOS-008** When the callee is declared in the same file and the parameter that receives a shorthand closure is certainly not a function (a number, string, list, option, ...), the error explains the scoping rule and shows the explicit closure to write: `group(key(_.clone()))` closes over `key`'s argument, not over `key(..)`.

<!-- example: tests/spec/closures/SPEC-CLOS-008.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn key(s: string) -> string {
    s
}

fn group(f: fn(string) -> string, s: string) -> string {
    f(s)
}

fn main() {
    let r = group(key(_.clone()), "a")
}
```
Error contains: `shorthand makes a closure here`

## Boxed closures, function pointers and capture

**SPEC-CLOS-009** A closure returned as `Box<dyn Fn>` keeps the values it captured with `move`; a function name is a value of function-pointer type; a closure that mutates a captured `mut` variable updates it in place; a closure captures the binding visible where it is written, so a later shadowing `let` does not change it.

<!-- example: tests/spec/closures/SPEC-CLOS-009.wj -->
```windjammer
fn make_adder(n: int) -> Box<dyn Fn(int) -> int> {
    Box::new(move |x| x + n)
}

fn apply_twice(f: fn(int) -> int, x: int) -> int {
    f(f(x))
}

fn double(x: int) -> int {
    x * 2
}

fn main() {
    let add5 = make_adder(5)
    println("${add5(1)} ${add5(10)}")
    println("${apply_twice(double, 3)}")
    let mut counter = 0
    let mut inc = || {
        counter += 1
        counter
    }
    inc()
    inc()
    println("${inc()}")
    println("${counter}")
    let base = 10
    let f = |x: int| x + base
    let base = 1000
    println("${f(1)} ${base}")
    let names = vec!["b", "a", "c"]
    let mut sorted = names.clone()
    sorted.sort()
    let joined = sorted.iter().map(|s| s.to_uppercase()).collect::<Vec<string>>().join("-")
    println(joined)
}
```
Output:
```text
6 15
12
3
3
11 1000
A-B-C
```
The compiled backend cannot build this program yet (docs/drift/spec-findings.md); the output above is that of the equivalent Rust program.
