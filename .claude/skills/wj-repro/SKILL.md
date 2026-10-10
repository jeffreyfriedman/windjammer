---
name: wj-repro
description: Turn a dogfood failure (wrong/non-compiling generated Rust) into a minimal, generic compiler regression test in tests/.
---
1. Reduce the failing `.wj` to the smallest single-file program that shows the bug; remove project names, domain types and dependencies.
2. Add `tests/bug_<behavior>_test.rs` (build.rs auto-registers it). Use existing helpers in `tests/` (see `tests/README.md`); assert on emitted Rust and, where relevant, that it compiles with `rustc --out-dir <tempdir>`.
3. If the bug only appears multi-file/whole-program, also add a multipass ("tip") test.
4. Run it, confirm RED for the right reason, then fix in the IR solver/signatures (never name-based heuristics).
5. Check `grep -rl` for an existing equivalent test; merge or delete duplicates.
