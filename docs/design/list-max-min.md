# List `max()` / `min()`: std API review

`std/rust_core.wji` declares `Iterator<T>::max(self) -> Option<T>` and `min`, so `nums.max()` type-checks in Windjammer
(the interpreter runs it), but a list lowers to Rust's `Ord::max(self, other)` and rustc rejects it (E0061).

1. Keep and fix: give lists `max() -> Option<T>` / `min() -> Option<T>` (lower to `.iter().max().cloned()`); matches the documented iterator form and `nums.max() ?? 0`.
2. Remove from the documented surface: leave `a.max(b)` for numbers only and make `nums.max()` a diagnostic with the fix `nums.iter-style: nums.fold(..)` or a loop.
3. Gate: `list_method_lowering_typechecks_test` rows `max` / `min` sit in `KNOWN_GAPS` until the owner picks.
