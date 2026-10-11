---
status: accepted
last_verified: 3406ef05bb35
---
# ADR-0014: Windjammer-native surface syntax (retire leaked Rust-isms)

- **Status:** Accepted (2026-10-11) for Phase 1 (print/assert/panic/`vec!`/`format!`/`dbg!`). Phases 2-3 are Proposed.
- **Date:** 2026-10-11
- **Related:** [ADR-0007](0007-target-tiers-conformance.md), [ADR-0004](0004-concurrency-surface.md), pipe operator (`docs/design/pipe-operator.md`)

## Context
Windjammer has no macros, yet ~1,230 `.wj` uses of `println!`, 307 `vec!`, 191 `format!`, 44 `assert!`
leak Rust's macro syntax. Plain-function forms (`println(`, `assert(`) already parse (579 / 178 uses), so
two spellings coexist. Other leaks: `Vec/HashMap/HashSet/Option/Arc<Mutex<>>` type names, `::` paths,
`.unwrap()`, `.iter().map().collect()`.

## Decision
Phase 1 (accepted): built-in functions, not macros; `!` forms deprecated.
- `print`, `println`, `panic`, `assert`, `assert_eq`, `assert_ne`, `todo`, `unreachable` are plain calls taking
  interpolated strings (`println("x = ${x}")`, `"${x:.2}"` specifiers). No `{}` format strings, no `format`.
- `stderr.println(..)` / `stdout.println(..)` (`println` = `stdout.println`); `debug(x)` replaces `dbg!` (prints
  value with source location, returns it).
- `[1, 2, 3]` is the list literal; `vec![..]` deprecated.
- These are language intrinsics in the IR, lowered per backend (Rust `println!`, Go `fmt.Println`, JS
  `console.log`, interpreter native), so output is identical across targets.
- Migration: both spellings accepted; `wj lint` warns on `!` forms; `wj fix` rewrites them (including
  `{}`-style format strings to interpolation). After one release the `!` forms become errors.

Phase 2 (proposed, separate acceptance): `List<T>`, `Map<K,V>`, `Set<T>`, `T?`, `Shared<T>`; `.unwrap()` lint
(`?`, `.or(default)`, `.expect("why")`); iterator chains via pipes with inferred collection.
Phase 3 (`::` to `.`): dropped; revisit only if needed.

Unchanged: explicit `let mut` (ADR-0002), `match`, `impl`, `Self`, `fn`, inferred `&`/`&mut`/derives.

## Consequences
Parser/lexer accept intrinsics as calls; analyzer/IR treat them as intrinsics; stdlib/docs/examples/tests
and all dogfood repos are migrated by `wj fix`; formatter normalises to the new forms.

## Phase 1 implementation notes (2026-10-11)
- One registry: `src/intrinsics/` (`Intrinsic` enum, `from_call_name` / `from_macro_name`, template helpers). The
  parser lowers the `!` spellings to the same `Call` node as the plain forms (`stderr.println` -> `eprintln`,
  `dbg!` -> `debug`); `format!` is the existing interpolation node. Rust, Go, JS and the interpreter lower from it.
- `"${x:spec}"` accepts Rust-style specs (`.2`, `?`, `>8`, `04`, `x`...). A string literal with braces and no
  interpolation prints verbatim.
- `[a, b]` is a growable `Vec` in `let` initialisers and `Vec` struct fields (like `vec![..]`); elsewhere the
  existing contextual array rules apply, and `let a: [T; N] = [..]` stays a fixed array.
- Diagnostics: W0007 (`name!(..)` / `vec![..]`), W0008 (`{}` format string), parse-time warnings and `wj lint`, with
  span + replacement. `wj fix [--check] [--docs] <path>` (script: `scripts/wj-fix-surface.sh`) is idempotent and
  never guesses: `vec![..]` in call-argument/assignment position and dynamic `{:w$}` specs are reported for manual
  migration. See `docs/design/surface-syntax.md`.
