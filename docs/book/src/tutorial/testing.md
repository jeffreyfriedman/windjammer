# Testing
<!-- feature: doctests -->

Tests are ordinary functions that call `assert_eq` and friends. They live in files whose names end in `_test.wj`, so a
module `geometry.wj` keeps its tests in `geometry_test.wj` beside it.

## Writing tests

Mark a function with `@test`, or name it `test_something`. A failing assertion fails that test and the others still run:

```wj,test
fn square(x: int) -> int {
    x * x
}

@test
fn squares() {
    assert_eq(square(4), 16)
}

fn test_negative() {
    assert_eq(square(-3), 9)
}
```

The assertions are plain functions: `assert(condition)`, `assert_eq(left, right)` and `assert_ne(left, right)`. All three
accept an optional message as the last argument.

## Tables of cases

`@test_cases` runs one function for each row of a table, one parameter per column. A single test body covers a whole
family of inputs:

```wj,test
fn square(x: int) -> int {
    x * x
}

@test_cases([(2, 4), (3, 9), (-4, 16)])
fn table(x: int, expected: int) {
    assert_eq(square(x), expected)
}
```

## Running them

`wj test` finds the tests in a directory and runs them. With `--interpret` there is no build step, so the cycle from edit
to result is as short as running a program:

```sh,verify
$ cat > calc_test.wj <<'EOF'
fn square(x: int) -> int {
    x * x
}

@test
fn squares() {
    assert_eq(square(4), 16)
}

@test_cases([(2, 4), (3, 9)])
fn table(x: int, expected: int) {
    assert_eq(square(x), expected)
}
EOF
$ wj test --interpret .
squares ... ok
table ... ok
test result: ok. 2 passed; 0 failed
```

A failure shows the two sides of the comparison and the line that failed, with the source around it:

```sh,verify
$ cat > broken_test.wj <<'EOF'
fn square(x: int) -> int {
    x * x
}

@test
fn broken() {
    assert_eq(square(2), 5)
}
EOF
$ wj test --interpret . 2>&1 || true
broken ... FAILED
left: 4
right: 5
test result: FAILED. 0 passed; 1 failed
```

`--list` shows what would run without running it, and `--filter NAME` runs just the tests whose names contain `NAME`. Add
`--watch` to re-run on every save. Without `--interpret`, `wj test` compiles the tests with `cargo`, which is what a CI
run of a release build wants.

For machine consumption, `wj test --json` prints each result with its file, name and message.

## Examples in documentation comments

A fenced example inside a `///` doc comment is a test too. `wj test` runs every one, in the module they document, so an
example can call the function it sits on; `// expect:` lines are the expected output:

```sh,verify
$ cat > double.wj <<'EOF'
/// Doubles a number.
///
/// ```wj
/// // expect: 42
/// fn main() {
///     println("${double(21)}")
/// }
/// ```
pub fn double(n: int) -> int {
    n * 2
}
EOF
$ wj test --doc .
doctests: 1 passed, 0 failed
```

`wj test --doc` runs only the doctests, and `wj test --compiled` runs each one through the Rust backend as well and
requires both outputs to match.

## Flaky tests, found and replayed

A test that passes "usually" is a seed you have not found yet. In the deterministic world of the simulator, time is
virtual, random numbers and task scheduling come from a seed, and files live in memory. Try many seeds at once:

```text
wj test --interpret --simulate --seeds 1..500 --faults 0.1 .
```

The first failing seed is printed together with the command that reproduces it, and the same failure comes back every
time you run that command. `--faults` adds seeded input and output failures, so error paths get exercised too. See
[Concurrency](concurrency.md) for the scheduler and [Effects and capabilities](effects.md) for the rest of the
deterministic world.

## Testing a module

Tests `use` the module they test like any other code. Keeping them in a sibling file means they are not part of the
program you ship:

```sh,verify
$ mkdir -p calc/src
$ cat > calc/wj.toml <<'EOF'
[package]
name = "calc"
version = "0.1.0"
EOF
$ cat > calc/src/mathx.wj <<'EOF'
pub fn clamp(x: int, lo: int, hi: int) -> int {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}
EOF
$ cat > calc/src/mathx_test.wj <<'EOF'
use mathx::clamp

@test
fn clamps_both_ends() {
    assert_eq(clamp(-5, 0, 10), 0)
    assert_eq(clamp(50, 0, 10), 10)
    assert_eq(clamp(5, 0, 10), 5)
}
EOF
$ cd calc && wj test --interpret .
clamps_both_ends ... ok
test result: ok. 1 passed; 0 failed
```

## Examples in documentation are tests too

A fenced example in a doc comment or a Markdown page is checked by the same tools. The book you are reading is tested this
way: [How this book is tested](../appendix/how-this-book-is-tested.md).
