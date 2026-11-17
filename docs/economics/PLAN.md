---
status: proposal
---
# Plan: two measured economic advantages before beta

> **Proposal: not implemented.** This plan states targets for the beta, not current behaviour.

- **Date:** 2026-11-16. Inputs: [DEFINITION.md](DEFINITION.md), [RESULTS.md](RESULTS.md), [ADVANTAGES.md](ADVANTAGES.md).
- **Goal:** at least two rows of RESULTS.md meet DEFINITION.md's *win* threshold, each protected by a gate that fails
  when the advantage regresses, before the compiler beta. Public docs may then state those two, with the ratio and the
  suite, and nothing else.

## The top three expected wins, plainly

1. **Verifying a change costs a fraction of what it costs in Rust** (C1): `wj run --interpret` against `rustc` + run.
   Measured: 0.24x the CPU on 58/58 agent-sized programs (RESULTS.md D2). The work is making it the loop agents
   actually use (pilot 6 agents ran compiled builds instead) and recording it on a quiet machine in seconds.
2. **A new Windjammer program builds with no third-party dependencies** (C2), like its Rust twin, instead of compiling
   69 crates first. Today this is Windjammer's largest economic loss; fixed, it is parity on cold builds and a
   win on "first build in a fresh sandbox" against any Rust project that pulls a runtime-sized dependency set.
3. **Ownership decided by the solver removes the clones a human (or agent) adds to satisfy the borrow checker** (C3):
   from 156 clones to the twin's level on `dx`, then below it on agent-written programs, where clones are common.

The fourth, cheap, add-on: the compiler-owned release profile (C4) makes shipped binaries smaller than
`cargo build --release` defaults.

## Steps (agent-days, order, gate)

| Step | What | Agent-days | Depends on | Gate (must fail when the advantage regresses) |
|---|---|---:|---|---|
| 0 | Land this suite: `economics.wj`, `economics_report.wj`, twins, DEFINITION/RESULTS (this change) | done | - | CI: `economics.wj --static --check` (D7 counts, deterministic) and `economics_report.wj <run> --gate` (the claimed wins, D2 CPU <= 0.5 and D7 ceremony <= 0.5, hold in the committed run; fails when they do not) |
| 1 | Quiet-box run on the PC (Linux, perf counters), wall-clock D2/D3/D5 rows | 0.5 | 0 | results committed with load; RESULTS.md updated |
| 2 | **done** (wt-econ-c1 + wt-econ): `--verify` stops before the build on a panic; agent sheet names only it; D2 ratchet `bench/economics/d2_baseline.json` (0.368 cycles, 3% slack, `--d2 --check`) | 1.5 | 0 | D2 ratio check in the gate list; agreement suites (`compiled_agreement`, `engine_parity`) |
| 3 | `wj optimize --explain/--json/--pass/--list` (COMMANDS.md), promise entries + acceptance cases | 1 | 0 | `tests/promise_registry_test.rs`; mutation check on the report |
| 4 | C2a: generated `Cargo.toml` names the runtime, serde and smallvec only when the code references them | 1 | 0 | cold-build crate-count ratchet on 3 fixed programs (0 / <=2 / <=2) |
| 5 | C2b: runtime base dependencies behind features chosen by `runtime_features.rs`; `io/fs/env/process/thread/sync` compile with zero dependencies | 2 | 4 | same ratchet; full `dx` oracle in compiled mode; runtime feature-matrix check (`cargo check --no-default-features` per group) |
| 6 | C4: `codegen-units = 1`, `strip = "debuginfo"` in the generated release profile; D6 recorded | 0.5 | 4 | D6 ratio ratchet; shipped-run output checks |
| 7 | C3a: read-only non-Copy parameters solved to `Ref` (ADR-0001), call sites pass `&` | 3 | ADR-0001 solver stream | clone ratchet (`tests/optimizer_ratchet_test.rs` corpus and `dx` generated count); `impl Drop` refusal fixture; mutant that borrows a stored parameter |
| 8 | C3b: last-use move in the solver | 2 | 7 | same ratchet; liveness fixtures (later read, live borrow) |
| 9 | `wj economics --dims static`, then `verify` (COMMANDS.md) | 1.5 | 2 | promise cases; deterministic static numbers |
| 10 | C5: a parsing kernel whose idiomatic twin allocates per line; flip P8 on if the allocation win holds | 1 | 0 | allocation-count row; P8 mutant |
| 11 | Pilot 9 (paid): R vs interpreted-first with C1 in place, 10 tasks x 3 trials | 1 + spend | 2 | D1 cost per passing program with CI; acceptance: Windjammer CI upper bound below Rust's mean |

Status after step 0 (measured, Mac, wj 37d50458ec6a): two rows already meet their win threshold and are gated:
D2 verify loop on `dx` (0.24x CPU) and D7 ceremony (0.45x). What remains for "two advantages before beta" is to keep
them (gates), confirm D2 in seconds on the quiet PC (step 1) and make agents use the loop (step 2), and to remove the
largest losses (C2, C11) so the advantages are not cancelled by the first `cargo build`.

Total for steps 1-6: about 6.5 agent-days. With C3 (steps 7-8): about 11.5. C11 (idiom lowering, 3-4 agent-days)
and C12 (location-independent single-file builds, 1 agent-day) slot in after step 6.

Order rationale: C1 and C2 need no solver change and their measurements already exist; C3 depends on the ADR-0001
solver migration and is scheduled with that stream, not against it. Step 11 is the only paid step and comes last,
when the cheap levers are in.

## Kill / re-plan criteria

- C1: if the quiet-box D2 wall-clock ratio is above 0.5 on `dx` after step 2, profile interpreter startup
  (ADR-0038 rungs) before anything else.
- C2: if a referenced runtime module turns out to need a heavy dependency transitively, split the module rather than
  re-adding the dependency to the base set.
- C3: if removing clones does not move D5/D4 rows on `dx` (inputs are small), add `dx`-style kernels with realistic
  input sizes before claiming a run-time win; the clone count alone is not an economic claim.
