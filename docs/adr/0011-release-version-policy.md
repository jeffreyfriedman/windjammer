# ADR-0011: Release and version policy across repos

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [VERSIONING.md](../VERSIONING.md), `Cargo.toml` (`[workspace.package] version = "0.50.0"`), `CHANGELOG.md`, `.github/workflows/{release,publish}.yml`, [PUBLISHING.md](../PUBLISHING.md)

## Context
The workspace (compiler, `windjammer-runtime`, `windjammer-lsp`, `windjammer-mcp`) shares one version (`0.50.0`). Other repositories
(`windjammer-ui`, the game engine, the database, dogfood applications) live beside it and pin the compiler in different ways
(`docs/DOWNSTREAMAPP_WJ_PIN.md` documents one). Before this change three documents gave three different versions and policies
(`VERSIONING_POLICY.md` said 0.46.0 with separate versioning, `VERSIONING_STRATEGY.md` described a 0.1.0 plan, the README said 0.38.6).
Breaking changes happen in minor versions while the project is 0.x; there is no stated rule for stdlib/runtime compatibility, for the minimum
`wj` version a `wj.toml` can require, or for how generated-code expectations are versioned.

## Options
### A. Lockstep core, ranged ecosystem
Compiler, runtime, stdlib (`std/`), LSP and MCP always share the workspace version. Ecosystem repos declare `wj-version = "0.50"` (compatible
range, minor-pinned while 0.x) in `wj.toml` and CI-test against the compiler's release candidate. Breaking language changes only on minor bumps with a CHANGELOG "Breaking" section and a migration note.
### B. Independent versions everywhere
Pros: flexible. Cons: compatibility matrix nobody tests; the current drift.
### C. Calendar versioning
Pros: no breaking-change semantics. Cons: hides compatibility; unusual for Cargo consumers.

## Recommendation
Option A. A single `docs/VERSIONING.md` holds the rule. `wj.toml` gains a required-compiler field; `wj` refuses to build a project requiring a newer minor. 1.0 is gated on ADR-0001, ADR-0005 and the conformance contract (ADR-0007), not on a feature count.

## Consequences
- Two versioning docs merged into `docs/VERSIONING.md` (done in this change).
- Release workflow publishes crates in dependency order and attaches the conformance report.
- Downstream repositories need a compatibility CI job against `next/*` branches.

## Decision needed from: project owner
Approve A, the `wj-version` manifest field name, and the 1.0 gate.
