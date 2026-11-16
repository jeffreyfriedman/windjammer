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
| 4 | Dependencies: path/git build in dependency order, lock file; registry deferred | Closed for path and git | `wj.lock` v2 (whole graph, dependencies first, revisions and content hashes), `wj lock update/check`, `wj build --locked`, reproducibility gate `scripts/wj/lock_gate.wj`; the registry stays deferred | [ADR-0010](adr/0010-package-source-of-truth.md), [WJ-PKG-01](proposals/rfcs/WJ-PKG-01-package-management.md) |
| 5 | Concurrency: one coherent surface across backends; `async expr`/`spawn expr` do not parse; `go`/`chan::new()` were advertised but never existed | Decided, in progress | `AsyncCall`/`SpawnCall` never constructed; `thread {}`, `async fn`, `<-` do parse | Decided in [ADR-0004](adr/0004-concurrency-surface.md) (accepted); implementing [WJ-CONC-01](proposals/rfcs/WJ-CONC-01-async-concurrency.md) as amended by the ADR, in another work stream |
| 6a | Taint errors fail the build | Closed | `WJ0173` from `src/check/taint/`: `wj check`, `wj build`, `wj run` stop on untrusted data reaching a command, a file path or SQL text; labeled oracle cases (`tests/taint`) and a precision gate over tests, examples, scripts and the ecosystem (`scripts/wj/taint_precision.wj`) | [ADR-0009](adr/0009-security-enforcement.md) amendment |
| 6b | `.wj-capabilities.lock` | Closed | Wired by `wj caps show|approve`, `wj check`, `wj build` and the interpreter (`src/ir/effect_facts/lock.rs`, `src/cli/caps.rs`); the unused legacy module is deleted | ADR-0021 |
| 6c | Unknown external calls have no effect classification | Closed for std and `extern fn` (an undeclared extern performs `ffi`, WJ0044); effects are checked against `[capabilities]` by `wj check`, `wj build` and `wj run` | `src/ir/effect_facts/`, `src/check/effects.rs` | ADR-0021 |
| 7a | Go and JavaScript backends do not consume the IR | Open | `let _ir = ...prepare_ir_context(...)` in both generators | Plan WP 11; [ADR-0007](adr/0007-target-tiers-conformance.md) |
| 7b | Conformance on every backend | Measured | `scripts/wj/target_tiers.wj` runs `tests/conformance` and the spec run cases on rust, javascript, typescript, go and the interpreter; results, a shrink-only ledger and [TARGET_TIERS.md](TARGET_TIERS.md) are generated and gated (`target_tiers.wj check`). No backend is Tier 1 yet; wasm has no headless runner | [ADR-0007](adr/0007-target-tiers-conformance.md) |
| 7c | Stdlib is Rust-host-only; no portability tags | Open | `std/*.wj` over `windjammer-runtime` | [ADR-0008](adr/0008-stdlib-portability.md) |
| 8 | Optimizer: post-codegen pass shipped; economics claims unmeasured | Decided, in progress | `src/optimizer/` runs after codegen (ADR-0006 option A); no `wj optimize`/`wj economics` command yet | Decided in [ADR-0006](adr/0006-optimizer-fate.md) (accepted, option A); the economics measurement runs in another work stream; [WJ-PERF-01](proposals/rfcs/WJ-PERF-01-economic-efficiency.md) |
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
| Mixed-width integer arithmetic (`int + i32`) passes `wj check` and runs in both modes, although ADR-0015 asks for an explicit conversion | recorded as Q-017 in [spec/OPEN_QUESTIONS.md](spec/OPEN_QUESTIONS.md) | `tests/spec/` SPEC-TYP-003 |
| Integer literals without a type, captured by a closure or passed through tuples and options, can be typed `i32` on the compiled path while `int` is `i64` (the interpreter has one integer type); annotating the binding (`let offset: int = 10`) avoids it | the "Integer literal typing" family in [drift/findings.md](drift/findings.md) and `rust-int-literal-width-inference` in [reference/targets.md](reference/targets.md) | `tests/drift_corpus/repros/` |
| A struct defined in a file module and only reached through a function that returns it (`let items = make()` in `main`, without `use shapes::Item`): interpreted it runs, compiled it fails with `WJ0003` (`expected module_struct::Item, found shapes::Item`); importing the type with `use shapes::Item` is the workaround | `wj run --compiled` on the repro project | `tests/book_repros/module_struct/` |
| A `defer` inside a loop body runs at the end of each pass when compiled and never when interpreted | `wj run --interpret` vs `--compiled` on the repro | `tests/book_repros/defer_in_loop.wj` |
| `wj build main.wj` on a lone script (no `wj.toml`) whose `main.wj` declares a sibling file module (`mod counter`) emits only `main.rs`: it declares `mod counter;` but no `counter.rs` is written (rustc E0583), and a literal passed to the module's method is typed `2_i32`. `wj build <dir>` and `wj run --compiled main.wj` (which builds the directory) are right; a tool that builds the entry file alone (the width predictions of `wj check` do) reads this reduced program | `wj build --no-cargo -o out main.wj` on the repro | `tests/book_repros/lone_script_mod/` |
