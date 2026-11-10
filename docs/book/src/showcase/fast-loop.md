# A fast edit-run loop

<!-- feature: showcase-fast-loop -->

The time between saving a file and seeing what it does decides how you work. `wj run` interprets, so there is no build in
that gap. The numbers below are measured by a script in the repository, `scripts/book/measure_startup.wj`, and written to
`docs/generated/measured.json`; the page reads them from there, so they are never typed by hand.

The script runs a small program with `wj run --interpret` and, for comparison, builds and runs the same program as a fresh
compiled project. It records CPU time (user plus system, child processes included), which stays meaningful on a busy
machine, and it refuses to record at all when its own repeated runs disagree by more than a factor of three.

CPU milliseconds for an interpreted run, median of the repeated runs:

<!-- wj:gen name=metric key=book.startup.interpret_cpu_ms -->
20
<!-- wj:end -->

CPU milliseconds for building and running the same program as a new compiled project (the runtime libraries are already built and cached, so this is the cost of one more project, not the first build ever):

<!-- wj:gen name=metric key=book.startup.compiled_cpu_ms -->
350
<!-- wj:end -->

Measured on:

<!-- wj:gen name=metric key=book.startup.machine -->
Darwin arm64
<!-- wj:end -->

A compiled build is the right choice for shipping, and `wj build --release` gives you the optimised binary. The interpreter
is the right choice for the loop in between: running a script, trying a change, re-running tests.

```sh,verify
$ cat > quick.wj <<'EOF'
fn main() {
    println("first result, no build")
}
EOF
$ wj run quick.wj
first result, no build
```

Two flags keep the loop short as a program grows. `wj run --watch` re-runs on every save, and `wj test --interpret` runs the
tests without compiling them. The [interpreter or compiled](../tutorial/interpreter-or-compiled.md) chapter explains when to
switch, and [How the interpreter stays honest](trust.md) explains why the answers agree.
