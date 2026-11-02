---
status: proposal
---
# Diagnostic change requests from the agent pack

Source: `bench/agent-dx` pilots 2 and 3 (`results/pilot2`, `results/pilot3`, 240 Windjammer trials; counts below are trials whose recorded tool output contains the signal). Each request moves one pack tip into the
compiler's own message, so the 2 KB default sheet can shrink and agents who skip the sheet still get the fix at the moment of the error.
Tests named below are the proposed regression tests; every repro was run on the `next/v0.50.0` head of 2026-11-02.

## R0. `wj run --compiled` prints raw rustc errors (blocks R1-R3 for the most-used compiled command)

`wj run --compiled` runs `cargo run --quiet` with inherited stderr (`src/cli/run.rs`, lines 96-109, `cmd.status()`), so rustc diagnostics reach the
agent untranslated: `error[E0308]: mismatched types --> main.rs:60:17`, a generated-Rust span, rustc's own help text and a `rustc --explain` line.
`wj build` goes through the translator (`src/cli/build.rs` around line 391, `error_mapper::ErrorMapper`; `src/cargo_integration.rs::check_with_cargo`).
Request: capture cargo with `--message-format=json` in `run.rs` and render through the same `ErrorMapper` + `diagnostics::rustc_text::translate`
path as `wj build`, mapping spans back to the `.wj` file via the source map. Test: a `tests/` case running `wj run --compiled` on the R1 repro
and asserting stderr contains `WJ0003` and `.wj:` and does not contain `main.rs` or `rustc --explain`.
Evidence: 74 of 240 trials contained a raw `main.rs` span and 62 a `rustc --explain` line in a tool result.

## R1. `usize` / `int` width hint (pack tip: "`len()` is `usize` when compiled: `v.len() as int`")

Repro (interpreter prints 3, `wj build` fails):

```windjammer
fn main() {
    let v: Vec<int> = [1, 2, 3]
    let idx: int = v.len()
    println("${idx}")
}
```

Today: `mismatched types: expected `int`, found `usize`` with help "change the value or the declared type".
Change in `src/diagnostics/rustc_text.rs`, `Some("E0308")` arm (around line 66): before `wjify`, inspect the raw pair `(e, f)`.
- If one side is `usize` and the other `i64`/`i32`: keep the message and set help to
  `` `len()`, `count()` and indices are `usize` when compiled but `int` is 64-bit: convert with `as int` (for example `let n: int = v.len() as int`), or compare `i < v.len() as int` ``.
- If the pair is `i32`/`i64`: help `` give the variable an explicit type: `let x: int = ...` ``.
Same hint for the E0277 `can't compare` / `cannot add` forms (`Some("E0277")` arm, around line 124, currently WJ0017 "trait bound not satisfied"):
match `msg` containing `can't compare` or `cannot add` with a `usize`/`i32`/`i64` pair and use the same help. Also wire `wj explain WJ0003` (`src/error_catalog.rs`
E0308 entry, around line 125, whose solutions list is about string/int conversion) to include the `as int` example.
Tests: extend `rustc_text.rs` unit tests (`type_spellings_are_translated` style): `translate(Some("E0308"), "mismatched types", Some("expected `i64`, found `usize`")).help` contains `as int`; E0277 `can't compare `usize` with `i32``.
Pilot evidence: 6 trials show it in recorded output (pilot 2 stored only message heads), 9 of 12 re-built interpreted-first failures in pilot 2 (`docs/known-issues/agent-dx-compiled-len-to-int-binding.md`).
Better still: lower `len()` into an `int` binding with the cast in codegen so the error never fires; the hint is the fallback.

## R2. `string` / `char` / `&str` mismatch hint (pack tips: "use `string`, never `String`", "literals need no `.to_string()`")

Repro:

```windjammer
fn digit_char(d: int) -> string {
    if d < 10 { (d + 48) as u8 as char } else { (d + 55) as u8 as char }
}
fn main() { println(digit_char(3)) }
```

Today (raw through `run --compiled`): `expected `String`, found `char``, help "try using a conversion method". Through the translator the pair becomes `string` vs `char` with the generic help.
Change in the same E0308 arm of `rustc_text.rs`: when the raw pair contains `char` together with `String`/`&str`, set help to
`` `char` and `string` are different types: use `c.to_string()` to make a string from a character, or `s.chars()` to walk a string ``.
When the expected type is `()` and the found type is `String`/`&str` (seen in 5 trials: an `if` branch whose value is dropped), set help to
`` a block's value is its last expression: an `if` without `else` has no value, so add an `else` branch or end the branch with a statement ``.
Tests: two new `translate` cases as above. Evidence: 23 of 240 trials carried a String/`&str`/`char` mismatch (pilot 3 arm T, with no `wj docs`, hit it in 7 of 30).

## R3. `collect` element type (pack tip: none; agents learned it from the error)

Repros (both fail `wj build` today, both compile with `.map(|s| s.to_string())`):

```windjammer
fn main() {
    let line = " a b  c "
    let parts = line.split_whitespace().collect::<Vec<string>>()
    println("${parts.len()}")
}
```

```windjammer
fn main() {
    let line = " a b c "
    let parts: Vec<string> = line.trim().split(" ").collect()
    println("${parts.len()}")
}
```

Today: `a value of type `Vec<String>` cannot be built from an iterator over elements of type `&str``, falling into the `Some("E0277")` arm of `rustc_text.rs`
(WJ0017 "trait bound not satisfied", help "implement the trait for the type"), which is wrong advice.
Change: in the `Some("E0277")` arm, match `msg.contains("cannot be built from an iterator over elements of type")` first and emit
message `` cannot collect `&str` items into `Vec<string>` `` with help `` the iterator yields borrowed text: add `.map(|s| s.to_string())` before `.collect()` ``.
Better: fix the lowering (`.collect::<Vec<string>>()` and `trim().split(..).collect()` should insert the owning map; the plain `let p: Vec<string> = s.split(" ").collect()` already does).
Tests: `translate` unit test plus the two repros as compiled corpus programs. Evidence: 19 of 240 trials.

## R4. Lint messages already carry the fix (no change; pack can drop the "never `&`, `.iter()`, `.unwrap()`" detail)

W0001-W0006, W0010 state what to write. Pilot 2 arm C (no pack) hit them in 45 verify calls and recovered; arms with a sheet hit them in 0-4 per 30 trials.
Request only that `wj explain W0003` etc. keep the one-line replacement first (check `src/linter/codes.rs`), so the sheet bullet can become a pointer.

## Not a diagnostic: stdin

Reading input never errors (`io::stdin().read_line(..)` compiles and loops forever, see `docs/known-issues/agent-dx-stdin-apis-differ-between-modes.md`), so that tip must stay in the sheet
unless `std::io` gets one spelling that works in both modes.
