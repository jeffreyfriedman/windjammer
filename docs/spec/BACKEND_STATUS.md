# Backend status of the spec examples (measured)

`scripts/spec_backends.sh <wj> rust|js` compiles every `run`/`panic` example with the real compiler and runs it with `rustc` / `node`, comparing stdout and exit status with the example's `// expect:` oracle. `parse-ok` / `parse-error` examples are skipped (no front-end-only mode on compiled backends). Per-example results: [backend_status.tsv](backend_status.tsv). Measured with the `wj` built from this branch after merging next/v0.50.0 at f42c6764 (parser-gaps included); this is a snapshot, not a gate.

| Backend | Runnable examples | PASS | FAIL (built, wrong output) | BUILD (wj or toolchain rejected) |
|---|---|---|---|---|
| Rust (`wj build --no-cargo` + `rustc`) | 169 | 104 | 5 | 60 |
| JavaScript (`wj build -t javascript` + `node`) | 169 | 131 | 5 | 33 |
| Interpreter (`tests/spec_conformance_test.rs`) | all | all except the `xfail` examples | | |

Examples marked `xfail` are expected not to pass anywhere (the spec states the decided behaviour). The interesting rows are the **non-xfail** examples that fail on a compiled backend, because the spec and the interpreter agree and the backend does not.

**Rust, non-xfail failures (51):** ATTR-001, ATTR-003, ATTR-004, ERR-002, ERR-003, ERR-005, EXEC-002, EXEC-004, EXEC-005, EXEC-007, EXPR-004, EXPR-011, EXPR-018, GRAM-006, INTR-007, INTR-009, ITEM-006, ITEM-010, MOD-001, MOD-002, MOD-003, MOD-005, NUM-001, NUM-002, NUM-003, NUM-004, NUM-006, NUM-007, NUM-010, NUM-017, OWN-003, PAT-001, PAT-005, PAT-007, PAT-009, PIPE-005, PIPE-006, STD-001, STD-002, STD-003, STD-005, STMT-005, STMT-007, STMT-009, STR-007, STR-008, TRT-001, TRT-004, TYP-006, TYP-007, TYP-010.

**JavaScript, non-xfail failures (23):** CONC-005, ERR-006, EXEC-002, EXPR-011, EXPR-018, GRAM-006, GRAM-009, ITEM-006, MOD-001, MOD-003, PAT-006, PAT-007, STD-002, STD-004, STMT-007, STMT-008, STR-008, TRT-003, TRT-004, TRT-006, TRT-009, TYP-006, TYP-010.

Root causes found by reading the generated code (details and recommendations in OPEN_QUESTIONS.md): Q-027 (integer literals default to `i32` on Rust), Q-028 (float literals default to `f32` on Rust), Q-029 (`defer` is not implemented on Rust: the generator emits `// TODO: defer not yet implemented`), Q-030 (user functions named like intrinsics are ignored on Rust), Q-031 (nested module paths), plus the known `rust-int-literal-width-inference` and `String`/`&str` coercion issues listed in `docs/known-issues/`. The remaining JS BUILD failures are unimplemented constructs in the portable emitters (traits objects, nested patterns, modules); `wj` prints an explicit unsupported error rather than wrong code.
