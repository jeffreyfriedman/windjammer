# Compiled path (resolved): `let mut v = 0` that later mixes with `int` values was emitted as `i32`

Found by the loop-mode pilot (`bench/agent-dx`, arm A): 11 of 30 interpreted-first trials ended with an integer-width or `usize` mismatch in the generated Rust for a program that ran correctly under
`wj run --interpret` (tasks t006, t011, t013, t018). Annotating the accumulator fixed two of six re-built samples; the rest are other literal-typing spots of the same family.

```windjammer
fn main() {
    let a: int = 3
    let mut v = 0
    v = v * a + 1
    while v > 0 {
        v = v / a
    }
    println("${v}")
}
```

The interpreter prints `0`. `wj build` emits `let mut v = 0_i32` next to `i64` arithmetic and rustc reports

```
error[E0308]: mismatched types
  | while v > (0_i32 as i32) {
  |           -   ^^^^^^^^^^^^^^ expected `i64`, found `i32`
```

Workaround (used in the pack, `docs/agent/WINDJAMMER_FOR_AGENTS.md`): annotate numeric accumulators, `let mut v: int = 0`.
Fix location: integer-literal type defaulting in codegen (an untyped literal local whose uses are all `int` should be `i64`);
the same family as the `Option`/`str` literal findings in `agent-dx-compiled-str-literal-and-option-default-codegen.md`.
Guard: `bench/agent-dx/tests/test_compiled_pitfalls.py` (builds both spellings; the first assertion flips when the compiler is fixed,
which is the signal to drop the pack advice).

Resolved on next/v0.50.0 (the pilot-2 build): the repro builds and prints 0; the pack tip and its guard test were removed.
