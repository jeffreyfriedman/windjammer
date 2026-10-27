# Compiled path: `let idx: int = v.len()` is rejected by rustc (`usize` vs `i64`) while the interpreter accepts it

Found by the second loop-mode pilot (`bench/agent-dx/results/pilot2`): 9 of 12 re-built interpreted-first failures were this family
(t009, t011, t013, t006), the largest single cause of silent divergence after the int-accumulator fix.

```windjammer
fn main() {
    let v: Vec<int> = [1, 2, 3]
    let idx: int = v.len()
    println("${idx}")
}
```

`wj run --interpret` prints `3`; `wj build` fails with `error[E0308]: mismatched types ... expected i64, found usize`.
Workaround (pack, `docs/agent/WINDJAMMER_FOR_AGENTS.md`): `let idx: int = v.len() as int`.
Fix location: codegen should lower `len()` flowing into an `int` binding, argument or comparison with the cast (or `wj check` must
reject it, R5). Guard: `bench/agent-dx/tests/test_compiled_pitfalls.py` (the first assertion flips when the compiler is fixed).
