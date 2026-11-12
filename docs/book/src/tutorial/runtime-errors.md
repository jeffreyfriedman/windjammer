# Read a runtime error

When a program fails while it runs, `wj run` tells you where, how it got there and what the values were. One command's
output is usually enough to fix the failure; you do not need to add `println` calls and run again.

## The default report

This program asks for the sixth score of three:

```sh,verify
$ cat > pick.wj <<'EOF'
fn pick(scores: Vec<int>, which: int) -> int {
    let total = scores.len()
    scores[which]
}

fn main() {
    println("picking")
    println("${pick([90, 85, 70], 5)}")
}
EOF
$ wj run --interpret pick.wj 2>&1 | grep -v "^thread\|^note\|^warning"
picking
index out of bounds: the len is 3 but the index is 5
stack backtrace:
  --> pick.wj:3:11
 3 |     scores[which]
   |           ^
  0: `pick` at pick.wj:3:11
  1: `main` at pick.wj:8:20
locals in `pick`:
  scores = [90, 85, 70]
  which = 5
  total = 3
```

Read it from the top. The message says what went wrong and the excerpt shows the failing expression. The numbered lines
are the calls that led there: frame 0 is where it failed, and each later frame is the call it was in the middle of.
`locals` holds the values of the failing function at that moment. Here `which` is 5 and `scores` has three items, so the
fix is in the caller or in a bounds check. Output the program printed before the failure is still on stdout; the report
goes to stderr, and the exit status is 101, as for a compiled panic.

## As JSON

`--error-format json` prints the same facts as one line in the diagnostics schema, with `stack` and `locals`. It is
always the last line on stderr:

```sh,verify
$ cat > short.wj <<'EOF'
fn main() {
    let scores = [90, 85, 70]
    let which = 5
    println("${scores[which]}")
}
EOF
$ wj run --interpret --error-format json short.wj 2>&1 | tail -n 1 | grep -c '"code":"WJ0170"'
1
$ wj run --interpret --error-format json short.wj 2>&1 | tail -n 1 | grep -o 'redacted":false,"value":"5'
redacted":false,"value":"5
```

The members are described in `docs/DIAGNOSTICS.md` in the repository.

## Secrets and `--no-locals`

Values are shown, so a log of a failed run could hold them. A binding or field named like a credential (`password`,
`token`, `secret`, `api_key`), a string equal to an environment variable's value and a `Secret*` type show
`<redacted>`. To leave the values out altogether:

```sh,verify
$ cat > short.wj <<'EOF'
fn main() {
    let scores = [90, 85, 70]
    let which = 5
    println("${scores[which]}")
}
EOF
$ wj run --interpret --no-locals short.wj 2>&1 | grep -c "locals in" || true
0
```

The report is for the interpreter. A compiled program prints Rust's panic message and exits with the same status; to get
the stack and the values, run the program with `wj run --interpret`.
