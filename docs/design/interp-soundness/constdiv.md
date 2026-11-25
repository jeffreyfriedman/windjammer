---
status: current
last_verified: 5457283f4156
---
# Soundness: constant-divisor guards in the JIT (`int_arith`, resolved/jit/lower_expr.rs)

`Val.k` is `Some(c)` only for an untyped integer literal (`Literal::Int`, `Literal::IntSuffixed` that stays `Lit`) and for its
negation (`checked_neg`). Typed literals, `const` items, variables (even loop-invariant ones) and folded expressions have `k = None`:
no value that is merely observed gets a `k`. So `k = Some(c)` means the machine-code divisor operand IS the immediate `c` on every execution.

Rules (divisor `y`, dividend `x`):
- Zero guard (`y == 0` bail): emitted unless `k = Some(c)` with `c != 0`. `c == 0` keeps the guard (same trap, message, location, resume).
- MIN/-1 guard (`x == i64::MIN && y == -1` bail, signed path only): emitted unless `k = Some(c)` with `c != -1`. For `c == -1` it stays.
  `c == 1`, `7`, `-7`, `i64::MIN`/`MAX` literals cannot form the pair (`y != -1`), so the guard is dead.
- Widths: the signed path works on the i64 register value; the only i64-register overflow of `/` and `%` is `i64::MIN / -1`. A narrower
  type's `MIN / -1` (i32, i8, ...) yields a value outside the type: it is caught by `self.range(r, t)`, which is untouched. A `-1` divisor
  keeps the 64-bit guard anyway. `%` uses the same guard as `/` (the interpreter bails on `MIN % -1` for every width and the closure
  engine produces its result; the parity program covers it).
- Unsigned path (u8/u64/usize with a non-negative `Lit` or unsigned operand): only the zero guard exists (no overflow case); the
  rule is the same: skipped only for a `Lit` literal `c != 0`.
- Mixed unsigned/Lit combinations that take the `int_op` helper are untouched.
- `x % c == 0` rewrite (item 3): not done (report on pc-results).
