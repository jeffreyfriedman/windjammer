# ADR-0013: Bug-triage tracker

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** `tests/COMPILER_REPRO_QUEUE.md`, `tests/STDLIB_ADOPTION_QUEUE.md`, `tests/LANGUAGE_SPEC_TESTS.md`, `AGENTS.md` (bd workflow), `docs/known-gaps.md`

## Context
Compiler bugs found by dogfooding are recorded in `tests/COMPILER_REPRO_QUEUE.md`, a single markdown file that keeps growing (entries numbered
`P3.798` and rising, each with gate commands and "do not steal" lists). `AGENTS.md` mandates `bd` (beads) for task tracking. Plan and protocol
files add a third place (`docs/known-issues/<slug>.md`). The result: no single answer to "what is open?", queue entries embed commit hashes and gate
results that go stale, and the file is large enough to be expensive for agents to load. Nothing says when an entry is removed after a fix, or which entries
must have a regression test.

## Options
### A. `bd` is the source of truth; the queue is generated or archived
Every bug is a bd issue with labels (`area:codegen`, `class:ownership`, ...). Each fixed issue links its regression test name. The queue file is
frozen, moved to `docs/archive/`, and only an auto-generated summary remains (or none).
### B. Markdown queue is the source of truth; bd tracks only work items
Pros: reviewable in PRs. Cons: merge conflicts, no status, unbounded growth.
### C. GitHub issues
Pros: public, searchable. Cons: a third system; agent workflow is bd-centric.

## Recommendation
Option A with a retention rule: a closed bug keeps its regression test as the record; the queue holds only open entries; entries older than one
release without a test are re-triaged or closed. `docs/known-gaps.md` holds architectural gaps only, not individual bugs.

## Consequences
- `tests/COMPILER_REPRO_QUEUE.md` (not owned by this change) is archived by the owner of `tests/`.
- AGENTS.md states "file bugs in bd, add a failing generic test first".
- Dogfood projects report through bd, not by editing compiler docs.

## Decision needed from: project owner
Choose A, B or C and the retention rule.
