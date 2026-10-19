# Compiled: an index variable used for `v[i]` is inferred `usize` even when `i + 1` is pushed into a `Vec<int>` or added to an `int`

```windjammer
fn line_numbers(lines: Vec<string>) -> Vec<int> {
    let mut out = Vec::new()
    let mut i = 0
    while i < lines.len() {
        out.push(i + 1)
        i = i + 1
    }
    out
}

fn total(a: Vec<string>) -> int {
    let mut n = 0
    for s in a {
        n = n + s.len()
    }
    n
}

fn main() {
    let v = ["a", "b"]
    println("${line_numbers(v).len()} ${total(v)}")
}
```

`wj build` emits `let mut i = 0_usize` and `out.push(i + 1_usize)` (rustc `E0308: expected i64, found usize` because the
function returns `Vec<int>`); `total` fails the same way (`n += s.len()` where `n` is inferred `i64` but `len()` is `usize`).
`int` is the language integer, so `.len()` and loop counters used as `int` should be converted (or inferred as `int`)
consistently. See also `ownership-and-width-ratchet-residue.md`.

Status: handed to the convergence / numeric-solver stream. Ratchet: `ecosystem_findings_handoff_test::index_counter_pushed_to_vec_int_keeps_one_width` and `len_into_int_field_converts`, listed in `.config/known-test-failures.txt`. Blocks the ecosystem `wj-grep` app (`out.push(i + 1_usize)` into `Vec<int>`).

Second error behind the width one (ecosystem `wj-grep`, `adapters/runner.wj`): after the width is fixed the app still fails with `E0382: use of moved value
replacement ... in previous iteration of loop`. `pub fn run(cmd: Command)` is emitted with an owned `cmd` (pub-API restore of an analyzed-borrowed formal), but its
body is generated as if `cmd` were a borrow: `match cmd.replace { Some(replacement) => .. }` inside `for root in &cmd.paths { for path in files { .. } }` is
not emitted as `match &cmd.replace`. Same family as `ownership-by-value-store-threaded-through-adapter-becomes-mut-borrow.md`: the body must be generated
against the formal that is emitted. A minimal single-file or two-module reduction did not reproduce it (they stay borrowed end to end); the full app does
(`wj build src` in the ecosystem's `apps/wj-grep`). Handed to the convergence stream.
