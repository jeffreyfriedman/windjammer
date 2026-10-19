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
| 3d | `wj eject` on the main CLI | Partial | `eject` exists only on the older `windjammer` binary (`src/cli_args.rs`, `src/ejector.rs`) | ADR-0012 (move into `wj` or drop the promise) |
| 4 | Dependencies: path/git build in dependency order, lock file; registry deferred | Partial | `DependencySpec` has path/git; `src/package/` has manifest parsing only; no `wj.lock` | [ADR-0010](adr/0010-package-source-of-truth.md), [WJ-PKG-01](proposals/rfcs/WJ-PKG-01-package-management.md) |
| 5 | Concurrency: one coherent surface across backends; `async expr`/`spawn expr` do not parse; `go`/`chan::new()` were advertised but never existed | Decision needed | `AsyncCall`/`SpawnCall` never constructed; `thread {}`, `async fn`, `<-` do parse | [ADR-0004](adr/0004-concurrency-surface.md), [WJ-CONC-01](proposals/rfcs/WJ-CONC-01-async-concurrency.md) |
| 6a | Taint errors do not fail the build | Partial | Only `src/cli/json_diagnostics.rs` consumes `taint_errors` | [ADR-0009](adr/0009-security-enforcement.md) |
| 6b | `.wj-capabilities.lock` not wired | Open | `src/ir/capability_lock.rs` unused by the CLI | ADR-0009, ADR-0010 |
| 6c | Unknown external calls have no effect classification; effects checked only with `[app_capabilities]` | Partial | `src/output_generation.rs` | ADR-0009, ADR-0012 |
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
