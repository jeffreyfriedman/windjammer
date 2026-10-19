# Diagnostics

Windjammer reports problems as data first and prose second. One schema
([diagnostics-schema.json](diagnostics-schema.json), `schema_version` 2) is shared by
`wj check --json`, `wj build --json`, `wj test --json` (the `diagnostics` array) and the LSP.
Human output is a rendering of the same records.

## `wj check`

```
wj check [PATH] [--json] [--watch] [--fix]
```

Checks `.wj` files (a file, or a project directory) with the compiler front end only: lexer,
parser, lints, surface-syntax findings, name resolution, conservative semantic checks, the
ownership analyzer, numeric inference and the IR solvers (effects, taint, execution). It never
invokes cargo or rustc and never writes generated code. Exit status is 1 when any error is found.

| Flag | Meaning |
|------|---------|
| `--json` | print one `Report` object (schema below) instead of text |
| `--fix` | apply every `machine_applicable` fix, re-check, report what remains |
| `--watch` | re-check when any `.wj` file changes (mtime polling, 250 ms) |

### What the front end catches today (no rustc)

| Code | Situation |
|------|-----------|
| WJ0001 / WJ0002 | undefined value / function (with a "did you mean" fix) |
| WJ0003 | literal of the wrong type in `let`, call argument or struct field |
| WJ0005 / WJ0009 / WJ0014 | unknown struct, missing field, unknown field (literal, `x.f`, `self.f`) |
| WJ0006 | unresolved `use` |
| WJ0011 / WJ0012 | syntax error, invalid character (with line/column) |
| WJ0013 | wrong argument count |
| WJ0003 (conditions, assignments, calls) | non-`bool` `if`/`while` condition; assigning another type to a typed local; `let x: int = f()` where `f` returns `string` |
| WJ0004 | assignment to a binding declared without `mut` (warning until the ADR-0002 migration flips `rules::MUTABILITY_REQUIRED`; fix adds `mut`) |
| WJ0005 (annotations) | `let x: Name` where `Name` is declared nowhere |
| WJ0030 | `Enum::Variant` that the enum does not declare |
| WJ0010 | `match` over a user enum that misses variants |
| WJ0015 | unknown method (on a known user type, or a name no type has) |
| WJ0017 | `${x}` where `x` is a user type without `Display` |
| WJ0018 | `impl Trait for T` missing required methods |
| WJ0019 | call to a private function of another module |
| WJ0020 | `string + int`, arithmetic on strings, or declared `int` mixed with declared `float` |
| WJ0003 (return) | literal of the wrong type returned from a function |
| WJ0021 | duplicate top-level definition |
| WJ0023 | forbidden Rust pattern (`.as_str()`, redundant `@derive`) |
| WJ0024-0028 | effect, taint, execution-mode, numeric-inference and analysis errors |
| W0001-W0008, W0010, W0011 | Rust-leakage lints and ADR-0014 deprecations |

Moves, borrow conflicts, lifetimes and anything that needs expression types beyond the cases
above still need rustc. Those errors are translated back to
`.wj` source with WJ codes by `src/error_mapper.rs` + `src/diagnostics/rustc_text.rs`
(ADR-0005, option C). Each such code has `origin: "rustc_mapped"`: it is a front-end coverage gap,
tracked in `wj explain CODE` ("Detection: found by rustc and translated").
A rustc error with no translation becomes `WJ0099` ("internal: please report"), never raw rustc text.

## Schema

```json
{
  "schema_version": 2, "windjammer_version": "0.50.0", "command": "check", "success": false,
  "summary": { "errors": 1, "warnings": 0, "files_checked": 1, "files_cached": 0, "elapsed_ms": 3 },
  "diagnostics": [{
    "code": "WJ0001", "severity": "error", "category": "name_resolution", "origin": "front_end",
    "message": "cannot find value `count` in this scope",
    "primary_span": { "file": "main.wj", "byte_start": 61, "byte_end": 66,
                      "line_start": 4, "col_start": 24, "line_end": 4, "col_end": 29,
                      "label": "not found in this scope" },
    "secondary_spans": [], "help": ["a value with a similar name exists: `counter`"], "notes": [],
    "fixes": [{ "description": "replace `count` with `counter`", "applicability": "maybe_incorrect",
                "edits": [{ "file": "main.wj", "byte_start": 61, "byte_end": 66, "line_start": 4,
                            "col_start": 24, "line_end": 4, "col_end": 29, "replacement": "counter" }] }],
    "docs": { "explain": "wj explain WJ0001", "url": "https://windjammer.dev/errors/WJ0001" },
    "maturity": "proven"
  }]
}
```

* **Spans**: lines/columns 1-based (columns count Unicode scalar values); byte offsets 0-based, half-open.
* **Codes** are stable and never reused. `WJxxxx` are error classes, `Wxxxx` lints. `WJ0098`/`WJ0099` are internal.
* **Fixes** are text replacements. Applicability follows rustfix: `machine_applicable` (applied by
  `--fix`), `maybe_incorrect` (a plausible suggestion, never auto-applied), `has_placeholders`, `unspecified`.
  Overlapping fixes are applied one at a time (the later one is skipped); `--fix` never writes a file
  that no longer parses.
* **`wj explain CODE`** prints explanation, cause, fix and a wrong/corrected example. `wj explain E0308`
  accepts rustc codes too.
* `wj build --json` runs the front end first, then codegen, then (with `--check`) cargo with every rustc
  error translated. Exit status is non-zero on any error. `wj test --json` adds `schema_version`,
  `command` and `diagnostics` to its existing result object.

## Speed and caching

`wj check` caches the whole result under `.wj-cache/check-v1-*.json`. The key covers every `.wj`
path and content plus the compiler build, so any edit anywhere recomputes everything and an unchanged
project is answered without lexing. (Finer-grained reuse is not sound yet: name resolution and the
semantic checks look across files.)

Measured with the release `wj` on a machine under heavy load (load average 65-135, so wall times
are inflated 5-10x; CPU time is the trustworthy column):

| Input | Cold CPU | Cold wall (loaded) | Cached CPU |
|-------|----------|--------------------|------------|
| 1 file, 14 lines | 26 ms | 427 ms | 3 ms |
| 1 file, 236 lines | 20 ms | 269 ms | 5 ms |
| 23 files, 2450 lines | 117 ms | 1.2 s | 4 ms |
| 41 files, 4536 lines (`std/`) | 175 ms | 2.7 s | 4 ms |

Re-measure on a quiet machine with `python3`-timed runs of `wj check --json PATH` (the first run
is cold, later runs hit the cache; `files_cached` in the report says which).

## Quality gates

`tests/diagnostics_suite.rs` fails when:

* a code literal appears in `src/` without a registry entry, or an entry lacks cause / fix / example;
* a front-end example does not produce its code, or its corrected example still does;
* the rendered message of any case in `tests/diagnostics/cases/` changes (goldens in
  `tests/diagnostics/golden/`, regenerate with `UPDATE_GOLDEN=1`);
* `wj check` reports a semantic error on a corpus file that the real build accepts (false-positive sweep).

`scripts/diagnostics-leak-report.sh` measures how many cases still reach rustc and how many of those
come out untranslated (it builds the generated crates with cargo, so it is not part of the test run).

Result on the 47 cases in `tests/diagnostics/cases` (11 of them are errors only rustc finds today):

| | before | after |
|--|--------|-------|
| cases the front end rejects without rustc | 0 (`wj check` ran cargo) | 32 of 47 |
| cases that still leak a rustc error | every error case (all went through cargo) | 11 (14 rustc errors) |
| leaked errors with no WJ code (raw `E....`) | 10 of 14 | 0 of 14 |
| leaked errors without a `.wj` location | not measured (no source maps for single-file builds) | 0 of 14 (located by symbol search) |
| leaked errors showing rustc help/notes | all | 0 (dropped; WJ0099 keeps the original code as a note) |

"before" is the committed mapper, whose registry knew 11 rustc codes. Single-file and project builds
write no source maps, so mapped locations come from searching the `.wj` text for the identifier rustc
named; when none is found the location is marked approximate.

## Negative gate (check vs build + rustc)

`tests/check_negative_gate.rs` is a differential test against a recorded rustc oracle. Corpus: 131
type-error mutants of the `// rust: ok` verified examples (`scripts/gen-check-mutants.py`: string
into int, int into string, undefined variable / function, dropped / extra / wrong-typed argument,
wrong return literal, unknown field, unknown method), the diagnostics cases and the error-system
corpora. `scripts/check-negative-oracle.sh` records whether `wj build --no-cargo` + `cargo check`
accepts each program (`tests/check_negative/oracle.tsv`, keyed by a source hash; the test fails when
it is stale). Whenever build + rustc rejects a program, `wj check` must report an error, except the
entries in `tests/check_negative/known_false_negatives.txt` (ratchet: only shrinks). Programs rustc
accepts must get no front-end semantic error. A mutation self-test proves the gate fails for a
checker that reports nothing.

| | false negatives (of 180 rejected programs) |
|--|--|
| check that only parses, lints and transpiles (the previous `wj check`) | 169 (93.9%) |
| `wj check` now | 3 (1.7%): two borrow conflicts and one use-after-move |
