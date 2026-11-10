# Your first project

This chapter creates a project, runs it, checks it, looks at the Rust it compiles to, and tests it. Each command is
shown with the output you should see; the book's test suite runs them against the real `wj`.

## Create and run

`wj new` makes a directory with a manifest (`wj.toml`) and a starter `main.wj` in `src`. Replace the starter program with a first
one of your own and run it:

```sh,verify
$ wj new greeter
Project created successfully
$ cat > greeter/src/main.wj <<'EOF'
fn main() {
    let name = "World"
    println("Hello, ${name}!")
}
EOF
$ cd greeter && wj run
Hello, World!
```

`wj run` interprets the program: there is no build step, and a whole multi-module project starts in milliseconds.
Strings interpolate with `${...}`, and `println` is an ordinary function, not a macro.

## Try code quickly

You do not need a file to try an idea. `wj eval` runs a piece of Windjammer and prints the value of the last expression.
The common standard modules (`strings`, `math`, `json`, `time`, `fs`, `env`, and the collections) are already imported, and
statements can be separated by `;` or newlines:

```sh,verify
$ wj eval 'strings.to_upper("hi")'
HI
$ wj eval 'let x = 6; x * 7'
42
```

`wj run -` reads a whole program from standard input, which suits generated code and one-off scripts from a pipe:

<!-- promise: feature run-stdin -->
```sh,verify
$ echo 'fn main() { println("from stdin") }' | wj run -
from stdin
```

`wj eval` runs interpreted, like `wj run`. For a longer session, `wj repl` keeps definitions between inputs.

## Check

`wj check` reads your program and reports problems without running it:

```sh,verify
$ cat > check_me.wj <<'EOF'
fn main() {
    let total = 40 + 2
    println("${total}")
}
EOF
$ wj check check_me.wj
0 error(s)
```

<!-- promise: feature json-diagnostics -->
Add `--json` to get the same diagnostics as structured data, with stable codes and exact spans. Editors and coding
agents read that form.

## See the Rust

Every Windjammer program compiles to Rust, and `wj build --no-cargo` writes that Rust out without invoking `cargo`:

```sh,verify
$ cat > hello.wj <<'EOF'
fn main() {
    let name = "World"
    println("Hello, ${name}!")
}
EOF
$ wj build --no-cargo hello.wj -o out
$ cat out/hello.rs
println!("Hello, {}!", name);
```

Plain `wj build` goes on to run `cargo` and produces a native binary; add `--release` for an optimised one.
`wj run --compiled` builds and runs in one step. [Interpreter or compiled](../tutorial/interpreter-or-compiled.md)
explains when to reach for each.

## Test

Tests live in files named `*_test.wj`. A function marked `@test` is a test, and `wj test` runs them:

```sh,verify
$ cat > math_test.wj <<'EOF'
fn double(n: int) -> int { n * 2 }

@test
fn doubles_numbers() {
    assert_eq(double(21), 42)
}
EOF
$ wj test --interpret .
test result: ok. 1 passed; 0 failed
```

## What is in a project

A project is a directory with a `wj.toml` and a `src` folder. The file `main.wj` in it holds `fn main`; other files there
are modules. The manifest names the project, lists its dependencies, and says what the program is allowed to do (see
[Effects and capabilities](../tutorial/effects.md)). The full format is in [wj.toml](../reference/wj-toml.md).

## Next

The [tutorial](../tutorial/values-and-types.md) starts with values and types.
