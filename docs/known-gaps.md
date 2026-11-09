# Known gaps

Ranked list of what is missing or wrong relative to what the documentation, RFCs and README promise. Source: the architecture
audit in [ARCHITECTURE_IMPROVEMENT_PLAN.md](ARCHITECTURE_IMPROVEMENT_PLAN.md) section 6, re-verified against the code on 2026-10-10.
Individual compiler bugs are not listed here ([ADR-0013](adr/0013-bug-triage-tracker.md)).

Status values: **Open** (nothing wired), **Partial** (some of it works), **Decision needed** (blocked on an ADR).

| Rank | Gap | Status | Evidence (today) | Work / decision |
|------|-----|--------|------------------|-----------------|
| 1 | Ownership, clone and numeric-width correctness is not owned by one component; blocks every dogfood project | Partial | Analyzer, IR solver and post-IR patches all decide ownership; width decided in 11 places; Rust-only solver consumption | Plan WP 6-9; [ADR-0001](adr/0001-ownership-truth.md) |
| 2 | No native diagnostics: type/borrow errors come from rustc and are translated; `compile_error!` emitted for unresolved boundary signatures | Partial | `src/error_mapper.rs`; IR doc "missing boundary signature" | [ADR-0005](adr/0005-error-strategy.md); needs name resolution (WP 6-7) |
| 3a | `.wj` formatter | Open | `wj fmt` runs `cargo fmt --all` (`src/cli/fmt.rs`) | Formatter work package (in progress elsewhere) |
| 3b | Real `wj check` | Partial | `wj check [--json] [--watch] [--fix]` runs the front end only (`src/check/`); catches names, arity, literal types, fields, mutability, lints; moves, borrows, traits, unknown methods, match exhaustiveness and operand types still need rustc and are translated (`src/diagnostics/rustc_text.rs`) | ADR-0005; [DIAGNOSTICS.md](DIAGNOSTICS.md) |
| 3c | `wj watch`, `wj doc`, `wj publish` | Open | No such subcommands in `src/bin/wj_cli_args.rs` (`wj docs` generates error docs only) | Roadmap "Then" list |
| 4 | Dependencies: path/git build in dependency order, lock file; registry deferred | Partial | `DependencySpec` has path/git; `src/package/` has manifest parsing only; no `wj.lock` | [ADR-0010](adr/0010-package-source-of-truth.md), [WJ-PKG-01](proposals/rfcs/WJ-PKG-01-package-management.md) |
| 5 | Concurrency: one coherent surface across backends; `async expr`/`spawn expr` do not parse; `go`/`chan::new()` were advertised but never existed | Decision needed | `AsyncCall`/`SpawnCall` never constructed; `thread {}`, `async fn`, `<-` do parse | [ADR-0004](adr/0004-concurrency-surface.md), [WJ-CONC-01](proposals/rfcs/WJ-CONC-01-async-concurrency.md) |
| 6a | Taint errors do not fail the build | Partial | Only `src/diagnostics/` consumes `taint_errors` | [ADR-0009](adr/0009-security-enforcement.md) |
| 6b | `.wj-capabilities.lock` | Closed | Wired by `wj caps show|approve`, `wj check`, `wj build` and the interpreter (`src/ir/effect_facts/lock.rs`, `src/cli/caps.rs`); the unused legacy module is deleted | ADR-0021 |
| 6c | Unknown external calls have no effect classification | Closed for std and `extern fn` (an undeclared extern performs `ffi`, WJ0044); effects are checked against `[capabilities]` by `wj check`, `wj build` and `wj run` | `src/ir/effect_facts/`, `src/check/effects.rs` | ADR-0021 |
| 7a | Go and JavaScript backends do not consume the IR | Open | `let _ir = ...prepare_ir_context(...)` in both generators | Plan WP 11; [ADR-0007](adr/0007-target-tiers-conformance.md) |
| 7b | Conformance suite runs only the Rust path | Partial | `tests/conformance/run_conformance_tests.sh` | ADR-0007 |
| 7c | Stdlib is Rust-host-only; no portability tags | Open | `std/*.wj` over `windjammer-runtime` | [ADR-0008](adr/0008-stdlib-portability.md) |
| 8 | Optimizer disabled; performance and economics claims unmeasured | Decision needed | `src/compiler_database.rs` `optimize_program`; no `wj optimize`/`wj economics` | [ADR-0006](adr/0006-optimizer-fate.md), [WJ-PERF-01](proposals/rfcs/WJ-PERF-01-economic-efficiency.md) |
| 9 | RFC features (comptime, const generics, `owned`) | Not started | No tokens/syntax; only `ConstEval::Comptime` IR scaffolding | [ADR-0003](adr/0003-owned-keyword.md), [WJ-LANG-02](proposals/rfcs/WJ-LANG-02-mojo-feature-analysis.md) |

## Language-surface questions the code and docs disagree on
| Question | State | ADR |
|----------|-------|-----|
| Is `let mut` required? | Docs say yes; codegen infers `mut` for mutated locals and parameters | [ADR-0002](adr/0002-local-mutability.md) |
| Does ternary `c ? a : b` exist? | RFC says implemented; parser removed it | WJ-SYN-02 row in the [RFC table](proposals/rfcs/README.md) |
| Are `use ... as` aliases and `std::http.client` submodule syntax supported? | Old docs said "future"; not verified in tests | [MODULE_SYSTEM.md](MODULE_SYSTEM.md) |

## Found while writing the book
Compiler and diagnostic findings from writing [The Windjammer Book](book/src/SUMMARY.md). Each has a minimal program under `tests/book_repros/` and a test in `tests/book_test.rs` that fails when the finding is fixed, so this table cannot go stale.

| Finding | Evidence | Repro |
|---|---|---|
| `wj explain-ownership` reports a nonsense source column for field reads inside a string with two interpolations (for example `@8:17640014`) | `wj explain-ownership` on the repro | `tests/book_repros/explain_ownership_interpolation_span.wj` |
| Mixed-width integer arithmetic (`int + i32`) passes `wj check` and runs in both modes, although ADR-0015 asks for an explicit conversion | recorded as Q-017 in [spec/OPEN_QUESTIONS.md](spec/OPEN_QUESTIONS.md) | `tests/spec/` SPEC-TYP-003 |
| A generic function that copies an element out of a list (`let mut best = items[0]` with `T: PartialOrd`) runs in the interpreter, but the compiled path stops with `WJ0015` ("no method `clone` found for type parameter `T`"); adding `+ Clone` to the bound is the workaround | `wj run --compiled` on the repro | `tests/book_repros/generic_largest_needs_clone.wj` |
| A function in a directory module (`util/mod.wj`) that takes a list and only reads it: the interpreter runs the program, the compiled path rejects the call with `WJ0003` (expected `Vec<i64>`, found `&Vec<i64>`); the same function in a file module (`util.wj`) compiles | `wj run --compiled` on the repro project | `tests/book_repros/dir_module/` |
| Three collection idioms run interpreted and fail compiled: `v.filter(|x| ...).collect()` into `Vec<int>` (`WJ0029`; `.cloned()` before `.collect()` is the workaround), `v.find(|x| x > 4)` (`WJ0003`, closure sees `&i64`), and `counts.entry(w)` with `w` taken from an array of string literals (`WJ0003`; typing the array as `Vec<string>` is the workaround) | `wj run --compiled` on each repro | `tests/book_repros/iter_filter_collect.wj`, `iter_find_closure.wj`, `hashmap_entry_literal_keys.wj` |
| Integer literals without a type, captured by a closure or passed through tuples and options, can be typed `i32` on the compiled path while `int` is `i64` (the interpreter has one integer type); annotating the binding (`let offset: int = 10`) avoids it | the "Integer literal typing" family in [drift/findings.md](drift/findings.md) and `rust-int-literal-width-inference` in [reference/targets.md](reference/targets.md) | `tests/drift_corpus/repros/` |
| A struct defined in a file module and only reached through a function that returns it (`let items = make()` in `main`, without `use shapes::Item`): interpreted it runs, compiled it fails with `WJ0003` (`expected module_struct::Item, found shapes::Item`); importing the type with `use shapes::Item` is the workaround | `wj run --compiled` on the repro project | `tests/book_repros/module_struct/` |
| A string literal as the first thing inside `${...}` (`"${"item":<8}"`) is a parse error (`WJ0011`); binding the literal to a name first (or passing it through a function) avoids it | `wj check` on the repro | `tests/book_repros/interpolation_leading_literal.wj` |
| `map_err(|_| "literal")` into a `Result<int, string>` runs interpreted and fails compiled with `WJ0003` (a literal is a borrowed string in Rust; interpolating a value, as in `"bad number: ${text}"`, avoids it) | `wj run --compiled` on the repro | `tests/book_repros/map_err_literal.wj` |
| A `pub const` of a module read from that module's own function runs interpreted and fails compiled with `WJ0001` | `wj run --compiled` on the repro | `tests/book_repros/module_const.wj` |
| Optional chaining `a?.b` runs interpreted; the Rust backend rejects it with `WJ0099` ("not supported yet") | `wj run --compiled` on the repro | `tests/book_repros/optional_chain.wj` |
| Looking up a string key (`groups.get(first)` with `first` a `String`) and inserting into a `BTreeMap<string, string>` runs interpreted and fails compiled with `WJ0003` (expected `&string`) | `wj run --compiled` on the repro | `tests/book_repros/map_get_string_key.wj` |
| `async fn` with `.await` inside it runs interpreted; the Rust backend rejects the call with `WJ0099` (E0728, "`await` is only allowed inside `async` functions"): the generated function is not `async` | `wj run --compiled` on the repro | `tests/book_repros/async_await_compiled.wj` |
| `@pure` and `@uses(..)` on a function are checked by `wj check` but reach Rust as attributes, so a compiled run fails with `WJ0099` ("cannot find attribute `pure`") | `wj run --compiled` on the repro | `tests/book_repros/pure_attribute_compiled.wj` |
| A `defer` inside a loop body runs at the end of each pass when compiled and never when interpreted | `wj run --interpret` vs `--compiled` on the repro | `tests/book_repros/defer_in_loop.wj` |
