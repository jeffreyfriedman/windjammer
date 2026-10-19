# src/reduce/

## Purpose
`wj reduce`: automatic test-case reducer for Windjammer projects. Shrinks a failing program (many files, many
items) to a minimal one that still triggers the same bug, then anonymizes it so it can be shared.

## Inputs/Outputs
In: a project directory or `.wj` file, an oracle (`rustc-error E0308[:regex]`, `panic`/`crash`, `miscompile`,
`cmd '<shell>'`), output dir, budgets (`--max-steps`, `--time-budget`), `--jobs`, `--seed`.
Out: the minimal anonymized project, `repro.md` (oracle, reproduce command, files/lines before/after), and a
resumable state directory `<out>.reduce-state/` (un-anonymized; keep it private).

## Algorithm
Hierarchical delta debugging over the lossless token tree of `formatter::scan_lossless` (`toks.rs`):
1. strip comments; 2. drop files together with the `use`/`mod` lines that import them (`pass_files`);
3. per file, per nesting depth, ddmin the units of every bracket group: items, impl members, statements, match
arms, struct fields, enum variants, parameters, generic parameters, call arguments (`pass_units`, `ddmin`);
4. replace sites by smaller forms: arguments and right-hand sides -> literals, `Name<A, B>` -> `A`, drop
attributes (`pass_sites`). Rounds repeat to a fixpoint or until the budget runs out.
A candidate is kept only if (a) `wj check` reports no error code the original did not (parse errors included) and
(b) the oracle still holds (`engine`). Results are cached by project content hash.

## Invariants
- Deterministic: candidate order depends only on `--seed`; parallel evaluation reports the lowest-index success,
  so `--jobs` never changes the result.
- Never prints input source: compiler/oracle output is captured and discarded, logs carry counts only; the only
  source text written is the final output project.
- Anonymization (`anonymize`, default on) renames identifiers, strings, file path components consistently
  (`T1`, `f1`, `field1`, `m1`, `C1`, `v1`, `s1`), drops comments, then re-checks the oracle on the exact output.
  Items that cannot be renamed without losing the bug are found by bisection; if any of them is a user-declared
  name the run fails and writes nothing unless `--keep-names` (then a warning is printed and recorded).
- Language and std names (`names.rs`) are never renamed.

## Owns
Reduction search, anonymization, oracle specs and evaluation, `repro.md`.

## Never decides
Compilation semantics (it shells out to `wj build`/`wj run`/`cargo check`, or runs the user's command).

## Allowed imports
Layer `tooling`: `check`, `formatter`, `lexer`, `module_system`, `embedded_assets`. Contract: docs/MODULE_MAP.toml.

## Entry points
`reduce::run(&ReduceOptions)`; CLI in `src/cli/reduce.rs`.

## Tests
Unit tests per file; `tests/reduce_*_test.rs` (planted bugs in a ~100-file program, anonymizer property test over
`examples/verified`, mutation test, determinism, CLI); promise cases in `tests/promises/reduce.toml`.
