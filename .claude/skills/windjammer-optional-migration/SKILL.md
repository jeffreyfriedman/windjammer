---
name: windjammer-optional-migration
description: Use when running wj fix or migrating Option/unwrap_or code.
category: windjammer-language
---

# Migrate Option code with `wj fix`, then hand-fix the rest

Run `wj fix` for the mechanical Option rewrites, verify, then migrate by hand whatever it leaves alone. Check the output; do not assume a rewrite is safe.

- Types: an Option of T becomes `T?`. Left alone: references to Option, `impl` blocks for Option, and turbofish Option forms. A doubly nested Option only has its inner option rewritten.
- `recv.unwrap_or(d)` becomes `recv ?? d` only when `d` has no calls and the call is a whole operand: a let initializer, an argument, a return value, or a match-arm value.
- `recv.unwrap_or_else(|| e)` becomes `recv ?? e` under the same operand rule.
- `.unwrap()` is never rewritten, because panic and propagate differ. Decide by hand.
- `vec![]` in call-argument position needs a manual migration; it cannot be rewritten safely without type information.
- Verify: run the interpreter before and after the fix and confirm identical output, then run `wj fix` again and confirm it changes nothing (idempotent).
- Status: `?.` optional chaining runs on the interpreter, Go, JS and TS. The Rust backend rejects it with an explicit error until convergence step C2 lands. `r?.b` on a Result no longer means `(r?).b`.
