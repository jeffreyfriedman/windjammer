# Architecture Decision Records

ADRs record decisions that shape the compiler, the language surface or the ecosystem, together with the
options considered. They exist so that agents and contributors stop re-deriving (or silently contradicting)
choices that the code and the docs currently leave implicit.

**Every ADR below is `Proposed`.** None has been accepted. Each carries a recommendation and a
"Decision needed from: project owner" line; an ADR becomes `Accepted` only when the owner edits its status
and adds the date and any amendments. Until then the recommendation is guidance, not policy.

| ADR | Title | Recommended option | Status |
|-----|-------|--------------------|--------|
| [0001](0001-ownership-truth.md) | Ownership truth: IR solver vs analyzer | IR solver is the single writer; analyzer shrinks to fact collection | Accepted (2026-10-10) |
| [0002](0002-local-mutability.md) | Local mutability: inferred vs explicit | Explicit `let mut` (Rust-like); parameters/`self` modes stay inferred | Accepted (2026-10-10) |
| [0003](0003-owned-keyword.md) | The `owned` keyword | Do not add; ownership stays inferred | Proposed |
| [0004](0004-concurrency-surface.md) | Concurrency surface | One documented surface: `async`/`spawn` expressions per WJ-CONC-01; `thread {}` kept; `go`/channels not added | Proposed |
| [0005](0005-error-strategy.md) | Error strategy | Front-end checks first; rustc errors stay a mapped fallback | Proposed |
| [0006](0006-optimizer-fate.md) | Optimizer fate | Delete the dormant 15-phase optimizer; keep IR-level passes; rebuild on measurement | Accepted (2026-10-10) |
| [0007](0007-target-tiers-conformance.md) | Target tiers and conformance contract | Tier 1 Rust, Tier 2 WASM/JS, Tier 3 Go; conformance suite gates promotion | Proposed |
| [0008](0008-stdlib-portability.md) | Stdlib portability | Portable core (pure `.wj` + capability-tagged host modules) | Proposed |
| [0009](0009-security-enforcement.md) | Security enforcement level | Effects fail the build; taint fails by default; lock file per dependency | Proposed |
| [0010](0010-package-source-of-truth.md) | Package source of truth | Path and git first, lock file, registry later | Proposed |
| [0011](0011-release-version-policy.md) | Release and version policy across repos | Compiler/runtime/stdlib lockstep; ecosystem repos declare a compatible range | Proposed |
| [0012](0012-rust-interop-boundary.md) | Rust interop boundary | `extern` declarations plus `[rust-dependencies]`-style Cargo passthrough, no inline Rust | Proposed |
| [0013](0013-bug-triage-tracker.md) | Bug-triage tracker | One tracker (bd) as source of truth; repro queue generated or archived | Proposed |

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
