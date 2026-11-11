# Made for agents

<!-- feature: showcase-agents -->

A coding agent works in a loop: write, check, run, read the feedback, fix. Windjammer is built so every turn of that loop
is cheap and unambiguous. People benefit from the same things.

## Diagnostics are data

`wj check --json` prints each problem as a record with a stable code, an exact span, help text and, when the compiler is
sure, a fix expressed as an edit. The schema is published (`docs/diagnostics-schema.json`) and shared by the command line,
the language server and the test runner:

```sh,verify
$ cat > mutate.wj <<'EOF'
fn main() {
    let x = 1
    x = 2
    println("${x}")
}
EOF
$ wj check --json mutate.wj; echo "exit $?"
"schema_version": 2
"code": "WJ0051"
"applicability": "machine_applicable"
"replacement": "mut "
exit 1
```

The fix is a byte-exact edit with an applicability level, so a tool can apply `machine_applicable` fixes without reading
prose. `wj check --fix` does exactly that:

```sh,verify
$ cat > mutate.wj <<'EOF'
fn main() {
    let x = 1
    x = 2
    println("${x}")
}
EOF
$ wj check --fix mutate.wj
$ cat mutate.wj
let mut x = 1
$ wj run mutate.wj
2
```

## No Rust toolchain needed to check

`wj check` is the compiler front end alone: lexer, parser, name resolution, ownership, numeric inference and the effect
analysis. It never starts `cargo` or `rustc`, so the feedback arrives in milliseconds and works on a machine with nothing
else installed. With only the `wj` directory on the path:

```sh,verify
$ cat > ok.wj <<'EOF'
fn main() {
    println("checked without cargo")
}
EOF
$ env PATH="$(dirname "$(command -v wj)")" wj check ok.wj
0 error(s)
```

## Instant runs

`wj run` interprets, so the second half of the loop needs no build either, and `wj eval` answers a question in one line:

```sh,verify
$ wj eval '[1, 2, 3].len()'
3
```

[A fast edit-run loop](fast-loop.md) puts numbers on it.

## Looking things up offline

`wj docs` searches the standard library, the built-in methods and a library of verified examples without a network:

```sh,verify
$ wj docs hashmap entry
HashMap
```

`wj explain CODE` explains any diagnostic code in full, and the [diagnostic codes](../reference/diagnostic-codes.md) page
lists them all.

## A sheet small enough to read first

The agent sheet is a single page, written to fit in the first moments of a session:

<!-- wj:gen name=file_bytes path=docs/agent/WINDJAMMER_FOR_AGENTS.md -->
1846 bytes
<!-- wj:end -->

Its examples run in the same test suite as this book, and a size ratchet keeps it from growing. The full reference behind
it is searchable with `wj docs`, and `llms.txt` at the root of the repository indexes everything for tools that want a
map. An MCP server exposes check, run, test, format, documentation search and error explanation as tools.
