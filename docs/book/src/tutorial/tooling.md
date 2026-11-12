# Tooling: fmt, check, fix, run, verify and cache
<!-- feature: tooling -->

Windjammer's tools are subcommands of one program, `wj`, and they are built to be run in a loop: write, check, run, fix.
This chapter walks that loop. [Command line](../reference/cli.md) lists every flag.

## Format: `wj fmt`

`wj fmt` rewrites files into the standard layout. `--check` changes nothing and exits non-zero when a file would change,
which is the form to use in CI:

```sh,verify
$ printf 'fn main(){\nlet   x=1\nprintln("${x}")\n}\n' > messy.wj
$ wj fmt --check messy.wj || true
Would reformat messy.wj
$ wj fmt messy.wj
Formatted messy.wj
$ cat messy.wj
fn main() {
    let x = 1
    println("${x}")
}
$ wj fmt --check messy.wj
1 file(s) checked
```

## Check: `wj check`

`wj check` reads the program and reports every problem without running it. The interpreter and the compiler apply the same
checks, so a program that passes `wj check` is one `wj run` will start. Diagnostics carry a code, a location, a help line
and, when a tool can apply it, a fix. `wj explain CODE` prints the long explanation of any code.

A warning worth knowing is `W0017`. In a string, `$$` is an escaped dollar sign, so `"$${price}"` prints the text `${price}`
and does not interpolate. If you meant a dollar sign followed by the value, write `\$${price}`:

```sh,verify
$ cat > cost.wj <<'EOF'
fn main() {
    let price = 5
    println("cost: $${price}")
}
EOF
$ wj check cost.wj
warning[W0017]: `$${` prints `${` literally
  = fix (suggested): replace `$${` with `\$${`
```

## Fix: `wj fix`

`wj fix` applies the mechanical rewrites in place. It migrates older surface syntax (`println!`, `vec![]`) and adds the `mut`
of changed locals. A local that is assigned or changed through needs an explicit `mut`, a loop variable included
([Values and types](values-and-types.md#loop-variables-too) explains the rule). Here the loop variable is written but not
declared `mut`; `wj check` says so, and `wj fix` adds it:

```sh,verify
$ cat > bump.wj <<'EOF'
fn main() {
    let mut xs = [1, 2, 3]
    for x in xs {
        x = x * 10
    }
    println("${xs:?}")
}
EOF
$ wj check bump.wj 2>&1 | grep -m1 "WJ0051"
error[WJ0051]: `x` is assigned after its declaration but is not declared `mut`
$ wj fix --check bump.wj || true
$ wj fix bump.wj
bump.wj:3: `x` is changed: `mut` added (WJ0051)
$ wj run bump.wj
[10, 20, 30]
```

`wj fix --check` writes nothing and exits non-zero when a file would change, so the same command works as a gate.

## Run: interpreted, verified, compiled

`wj run` interprets by default: no build step, and the first line of output arrives at once. Three flags change what you
get:

- `--verify` interprets, then builds the program and runs the compiled version too, and compares the two. The interpreted
  output streams as the program prints; the comparison comes at the end, on stderr. Standard input is read once and given
  to both runs.
- `--compiled` builds and runs the compiled program. A compiled run prints only your program's output and real
  diagnostics: the build's own progress lines stay out of the way.
- `--error-format agent` (or `json`) changes how problems are reported, for tools that read them.

```sh,verify
$ printf 'fn main() {\n    println("one")\n    println("two")\n}\n' > v.wj
$ wj run --verify v.wj < /dev/null
one
two
wj verify: compiled build OK; the compiled run matches
```

### One line per problem, for programs that read them

Coding agents and scripts read diagnostics, and every extra line costs them. `--error-format agent` prints one compact line
per diagnostic: location, code, message, the first help and the fix.

```sh,verify
$ printf 'fn main() {\n    let price = 5\n    println("cost: $${price}")\n}\n' > cost.wj
$ wj check --error-format agent cost.wj
cost.wj:3:20: warning W0017
```

See [What it costs agents](../showcase/agent-cost.md) for the measurements behind these choices.

## The caches: `wj cache`

Compiled builds share one Cargo target directory, so the first build of a program warms the runtime for every other.
Analysis results are cached too, keyed by the content of your files and the compiler's build, so a second `wj check` of an
unchanged module skips the work and several checkouts of one project share it.

```sh,verify
$ wj cache status
Windjammer Cargo cache
$ wj cache prune
```

`wj cache prune` reclaims old incremental build data. It leaves alone anything a running build holds or wrote within the
last hour, and it is more aggressive when the disk is nearly full; `--aggressive` and `--force` widen it. `wj cache warm`
pre-builds the runtime, and `wj doctor` finds a stale or duplicate `wj` on your path and prints the fix.

## Test and replay

`wj test` runs your tests ([Testing](testing.md)), and `wj replay` steps through a recorded or simulated run
([Debug a failing run deterministically](time-travel.md)). Both use the same interpreter you run programs with, so the
loop stays fast.
