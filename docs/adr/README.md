# Architecture Decision Records

ADRs record decisions that shape the compiler, the language surface or the ecosystem, together with the
options considered. They exist so that agents and contributors stop re-deriving (or silently contradicting)
choices that the code and the docs currently leave implicit.

The table below is generated from each ADR's `Status` line, so it cannot drift. An ADR becomes `Accepted` only when the
owner edits its status and adds the date and any amendments; until then its recommendation is guidance, not policy.

<!-- wj:gen name=adr_index -->
| ADR | Title | Status |
|---|---|---|
| [0001](0001-ownership-truth.md) | Ownership truth: IR solver vs analyzer | Accepted |
| [0002](0002-local-mutability.md) | Local mutability: inferred vs explicit | Accepted |
| [0003](0003-owned-keyword.md) | The `owned` keyword | Proposed |
| [0004](0004-concurrency-surface.md) | Concurrency surface | Accepted |
| [0005](0005-error-strategy.md) | Error strategy: front-end checking vs rustc translation | Proposed |
| [0006](0006-optimizer-fate.md) | Optimizer fate | Accepted |
| [0007](0007-target-tiers-conformance.md) | Target tiers and conformance contract | Proposed |
| [0008](0008-stdlib-portability.md) | Stdlib portability | Proposed |
| [0009](0009-security-enforcement.md) | Security enforcement level | Proposed |
| [0010](0010-package-source-of-truth.md) | Package source of truth | Proposed |
| [0011](0011-release-version-policy.md) | Release and version policy across repos | Proposed |
| [0012](0012-rust-interop-boundary.md) | Rust interop boundary | Proposed |
| [0013](0013-bug-triage-tracker.md) | Bug-triage tracker | Proposed |
| [0014](0014-surface-syntax-rust-isms.md) | Windjammer-native surface syntax (retire leaked Rust-isms) | Accepted |
| [0015](0015-language-consistency-proposals.md) | Language consistency proposals (numerics, casts, paths, returns) | Accepted |
| [0016](0016-ownership-pins.md) | Pins: the surface for overriding inference | Proposed |
| [0018](0018-expressiveness-phase-2.md) | Expressiveness Phase 2 | Accepted |
| [0019](0019-process-model.md) | Lightweight processes, mailboxes and supervision (Elixir-inspired) | Proposed |
| [0020](0020-interpreter-execution-engine.md) | Interpreter execution engine (tree walker vs resolved/bytecode tier) | Proposed |
| [0021](0021-effects-and-capabilities.md) | Effects and capabilities | Accepted |
| [0022](0022-interpret-by-default.md) | `wj run` interprets by default; compiled is the release path | Accepted |
| [0023](0023-interpreter-rust-crate-bridge.md) | Interpreter host bridge for Rust code and Rust-crate dependencies | Proposed |
| [0024](0024-hot-reload.md) | `wj run --watch` and hot reload of function bodies | Proposed |
<!-- wj:end -->

## Template

Copy this into `NNNN-short-title.md` (next free number, four digits).

```markdown
# ADR-NNNN: Title

- **Status:** Proposed | Accepted (YYYY-MM-DD) | Superseded by ADR-XXXX | Rejected
- **Date:** YYYY-MM-DD
- **Related:** RFCs, code paths, other ADRs

## Context
What is true today (with file paths as evidence) and why a decision is needed.

## Options
### A. ...
### B. ...
(Pros, cons, cost, what each breaks.)

## Recommendation
One option, and the reasons.

## Consequences
What changes in code, docs, tests and user programs if adopted; what becomes forbidden.

## Decision needed from: project owner
The specific question(s) to answer.
```
