---
name: downstream-build-failure-triage
description: Use when a downstream build (wj-game, plugins) fails with many errors, or with one error pointing into compiler source.
category: dogfood-triage
---

# Downstream build-failure triage

Do not hand-edit downstream source to route around a compiler bug. First classify the errors, then reduce each class to a generic repro, then route each class to the owning stream.

1. Count errors by class (for example E0308 by f32/f64 vs &T vs T, E0596 mutation through &self, E0277 mixed-width arithmetic, E0583 missing module file). A rising total is exposure only when the new errors sit in classes an earlier error was masking. If a change makes a small repro that compiled before emit a new mismatch, that is a regression in the change: fix it before the next merge.
2. For each class, write a minimal single-behavior `.wj` repro with neutral names. Verify it on the current compiler before calling it a compiler bug.
3. Classes that do not reproduce in one file are usually cross-module or registry-dependent. Build a 3-5 file project repro before fixing them.
4. Route by owner: numeric literal widths to the numeric solver, ownership and clones to the IR solver, name resolution and aliases to the resolver. Add no codegen special cases. An untyped literal takes its default type only when nothing in its expression constrains it. A default applied before unification with a typed peer produces mixed-width expressions such as `2_i32 * seg + 1_i64`, so the fix belongs in the solver's unification, not in a codegen patch.
5. A full downstream transpile is slow (about 75 minutes under load). Iterate on reduced repros and re-run the whole program only after they pass.
6. A single error naming a symbol inside compiler source (E0425 in `src/`) means the integration HEAD is broken, not the downstream. Find the deleting commit with `git log -S'<symbol>'`, restore the symbol, and route the fix to the branch owner. Do not build downstream against an older `git archive` snapshot to dodge it; the workaround hides the break from everyone else. See windjammer-merge-verification.
7. After compiler fixes land on the integration branch, re-verify from a fresh build of the branch HEAD, not a pinned snapshot: build `wj` with its own target dir and `CARGO_BUILD_JOBS=2`, regenerate the downstream output from a clean `gen/`, rebuild, then report the error count by class against the previous baseline. Name the classes still open and any new generic repros added.

Reference: [references/triage-checklist.md](references/triage-checklist.md) has the checklist and report shape.
