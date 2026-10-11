# Technical debt

The architecture-level debt, its ranking and the work packages that retire it live in:

- [docs/ARCHITECTURE_IMPROVEMENT_PLAN.md](docs/ARCHITECTURE_IMPROVEMENT_PLAN.md) (what to change, in order)
- [docs/known-gaps.md](docs/known-gaps.md) (what is missing relative to the docs and RFCs)
- [docs/adr/README.md](docs/adr/README.md) (decisions blocking some of the work)

Individual compiler bugs are not tracked here; see [ADR-0013](docs/adr/0013-bug-triage-tracker.md).

## Current debt (measured 2026-10-10; numbers are ratchets and may only fall)

| Item | Evidence | Retired by |
|------|----------|------------|
| Ownership decided in at least five places, clones in at least three | plan section 1 | WP 9, ADR-0001 |
| Layers import each other (analyzer, ir, codegen, type_inference) | plan section 1 | WP 4, Q1 |
| Registries keyed by bare strings (`get_signature(&str)` about 469 call sites) | plan section 1 | WP 6, 7 |
| Hidden state: 7 `thread_local!`, process-global registries, behaviour-changing env toggles | `grep thread_local! src` | WP 1, 3 |
| Crate-wide `#![allow(dead_code)]` | `src/lib.rs:8` | WP 5 |
| Optimizer module disabled | `src/compiler_database.rs` (`optimize_program`) | ADR-0006 |
| Two shader stacks beside `src/wjsl` (`src/shader`, `src/codegen/wgsl`) | directory listing | WP 5, 12 |
| Go/JS backends compute the IR and discard it | `let _ir = ...` in `codegen/go/generator/mod.rs`, `codegen/javascript/mod.rs` | WP 11, ADR-0007 |
| `src/package/` is a stub resolver and registry client | file headers | ADR-0010 |
| `wj fmt` and `wj check` wrap cargo instead of operating on `.wj` | `src/cli/fmt.rs`, `src/cli/check.rs` | known-gaps 3 |
| Parser has AST nodes for `async expr`/`spawn expr` that nothing constructs | `grep AsyncCall src/parser` | ADR-0004 |

## Rule
Debt that is knowingly left in place needs a row above with evidence and a retiring work package or ADR. Resolved items are
deleted from the table, not struck through; history is in `git log` and [CHANGELOG.md](CHANGELOG.md).
