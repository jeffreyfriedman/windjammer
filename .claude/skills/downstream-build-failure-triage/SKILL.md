---
name: downstream-build-failure-triage
description: Use when a downstream build (wj-game, plugins) fails with many errors, or with one error pointing into compiler source.
category: dogfood-triage
---

# Downstream build-failure triage

Do not hand-edit downstream source to route around a compiler bug. First classify the errors, then reduce each class to a generic repro, then route each class to the owning stream.

1. Count errors by class (for example E0308 by f32/f64 vs &T vs T, E0596 mutation through &self, E0277 mixed-width arithmetic, E0583 missing module file). A rising total after a compiler fix is not a regression by itself: more of the tree compiling exposes more errors.
2. For each class, write a minimal single-behavior `.wj` repro with neutral names. Verify it on the current compiler before calling it a compiler bug.
3. Classes that do not reproduce in one file are usually cross-module or registry-dependent. Build a 3-5 file project repro before fixing them.
4. Route by owner: numeric literal widths to the numeric solver, ownership and clones to the IR solver, name resolution and aliases to the resolver. Add no codegen special cases.
5. A full downstream transpile is slow (about 75 minutes under load). Iterate on reduced repros and re-run the whole program only after they pass.
6. A single error naming a symbol inside compiler source (E0425 in `src/`) means the integration HEAD is broken, not the downstream. Find the deleting commit with `git log -S'<symbol>'`, restore the symbol, and route the fix to the branch owner. Do not build downstream against an older `git archive` snapshot to dodge it; the workaround hides the break from everyone else. See windjammer-merge-verification.

Reference: [references/triage-checklist.md](references/triage-checklist.md) has the checklist and report shape.
