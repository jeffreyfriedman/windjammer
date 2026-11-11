# Interpreter or compiled
<!-- feature: program-execution -->

<!-- promise: feature interpret-instant -->
<!-- promise: feature multi-backend -->
<!-- promise: feature interpreter-matches-compiled -->

Windjammer has two ways to run a program, and they are meant to give the same answers: for a program the interpreter accepts, its standard output, exit status and panic location match the compiled program's (see [the interpreter's contract](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/testing/INTERPRETER_CONTRACT.md)).

* The **interpreter** runs your source directly. There is no build step, a whole multi-module project starts in
  no build step, and it needs no Rust toolchain. It is the default for `wj run`.
* The **compiled path** transpiles the program to Rust and builds it with `cargo`. It produces a native binary and the
  full speed of optimised Rust. `wj build` and `wj run --compiled` take it, and `--release` turns the optimiser on.

## Which one to use

| You are... | Use |
|---|---|
| writing and trying things out | `wj run` (interpreter) |
| running tests in a tight loop | `wj test --interpret` |
| scripting, tooling, one-off tasks | `wj run` |
| exploring in a session | `wj repl` |
| shipping a program, or timing it | `wj build --release` |
| using Rust crates with no interpreter surface, `extern fn`s, or threads | `wj run --compiled` |
| checking the code you are about to ship | both, and compare |

`wj run` chooses for you: it interprets, and when it finds before the program starts that the program needs the
compiled path, it switches over and prints one line on standard error saying so. The flags let you decide yourself:

```sh,verify
$ cat > hello.wj <<'EOF'
fn main() {
    println("same answer either way")
}
EOF
$ wj run --interpret hello.wj
same answer either way
```

`--interpret` never falls back: if the program needs something the interpreter lacks, it stops with exit status 2 and
names the feature. `--compiled` skips the interpreter altogether.

## Same program, same output

The two paths share the language's semantics: integer overflow is checked in both, floats print the same way, `match`
is exhaustive in both. The book relies on this. Its test suite runs every example in the interpreter and then again
through the compiled path, and fails if the outputs differ.

```wj
// expect: 55
// expect: 3.5
fn fib(n: int) -> int {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    println("${fib(10)}")
    println("${(7 as float) / 2.0}")
}
```

When you are about to ship, run it both ways. The interpreter can accept code the compiler would reject, because
compiling adds the checks of the Rust compiler on top of Windjammer's own:

<!-- ignore: shows the order of commands for a project; the same commands are exercised by the verified examples in Getting started -->
```sh,ignore
wj run --interpret src/main.wj
wj build
wj run --compiled
```

If `wj build` rejects something the interpreter ran, the failure is worth a minimal bug report: `wj reduce` shrinks a
failing project to a small, anonymised reproduction. The current list of differences between the two paths is kept in
[Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md).

## Speed

The interpreter has no build step, and its start-up cost is measured in [A fast edit-run loop](../showcase/fast-loop.md). A
compiled program pays for a build once and then runs as optimised Rust. Which one finishes your program sooner depends on how
much work the program does, so measure the one you care about: if it spends its time waiting for files, the network or the
user, the build buys little; if it spends its time computing, build it.

## Targets beyond Rust

The compiled path has more than one destination. `wj build --target` takes `rust` (the default), `javascript` and
`wasm`, and a Go backend is part of the conformance suite. [Compile targets](../reference/targets.md) lists which
programs each backend runs, generated from the conformance suite, so the table is always current.

## Watching and hot reloading

`wj run --watch` re-runs the program on every source change. `wj run --hot` goes further: it keeps the program running
and swaps in edited function bodies while keeping its state, which suits long-running servers and simulations.
