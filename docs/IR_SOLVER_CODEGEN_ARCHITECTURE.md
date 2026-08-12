# IR and Solver-Driven Codegen Architecture

**Status:** Active migration (2026-07). Replaces heuristic ownership/borrow codegen with a unified constraint solver consumed by all backends.

## Problem Statement

Legacy codegen resolves call-site coercions through a fragile stack of heuristics:

- Signature registry metadata (often stale or converged incorrectly)
- Method-name lists and collection-key flags
- Sequential add-then-strip-then-re-add phases in Rust argument emitters
- Post-hoc `correct_legacy_output` patches

This causes ping-pong fixes (dogfood E0308s, engine multipass regressions) and does not scale across backends.

## Design Principle

**One solver decision per argument:**

```
(actual SafetyType, expected SafetyType) → CoercionKind → target-specific emit
```

No method-name lists. No backend-specific ownership guessing. The IR constraint solver is the single source of truth; backends consume resolved `SafetyType` values via `target_encodings`.

## Pipeline Overview

```
Parser AST
    ↓
Analyzer + SignatureRegistry  (cross-module callee resolution)
    ↓
constraint_gen                (per-expr + call-site TypeEquals / OwnershipIs)
    ↓
Unified Solver                (types, ownership, clones, regions)
    ↓
Domain solvers                (effects WJ-SEC-01, taint WJ-SEC-02, execution WJ-CONC-01)
    ↓
IrModule                      (IrFunction + IrNode tree + SafetyType per node)
    ↓
ir::coercion::compute_coercion
    ↓
target_encodings::encode_call_argument (per Target: Rust, Go, JS, WASM)
    ↓
Backend codegen emit
```

## Core Types

### SafetyType (`src/ir/safety_type.rs`)

Canonical type for every IR node:

| Field | Purpose |
|-------|---------|
| `base` | `BaseType` (Int, Float, String, Custom, …) |
| `ownership` | `OwnedType` (Owned, Ref, MutRef, Copy, Inferred) |
| `effects` | `EffectSet` |
| `taint` | `TaintStatus` |
| `const_eval` | Compile-time eligibility |
| `exec_mode` | async/spawn call-site mode |

### CoercionKind (`src/ir/coercion.rs`)

Target-agnostic coercion decision:

| Kind | Meaning |
|------|---------|
| `Identity` | No transformation |
| `Borrow` | Pass by shared reference |
| `MutBorrow` | Pass by mutable reference |
| `Clone` | `.clone()` or target equivalent |
| `Deref` | Copy deref (`*` in Rust) |
| `ToOwnedString` | String literal → owned string |
| `StripBorrow` | Remove spurious `&` |
| `NumericCast` | Cast to expected numeric class |

### IrFunction / IrNode (`src/ir/node.rs`)

- **IrFunction:** name, `param_types`, `return_type`, `body: Vec<IrNode>`, local var map, optimizations
- **IrNode:** expression tree node with `safety_type` and `constraint_var`

## Module Map

| Module | Role |
|--------|------|
| `src/ir/constraint_gen.rs` | AST walk → constraints (including call-site unification) |
| `src/ir/solver.rs` | Unified union-find solver |
| `src/ir/pipeline.rs` | `lower_to_ir`, `try_codegen_from_ir` |
| `src/ir/coercion.rs` | `(actual, expected) → CoercionKind` |
| `src/ir/target_encodings.rs` | Per-target emit (`encode_call_argument`, `apply_coercion`) |
| `src/ir/shadow.rs` | IR vs legacy analyzer parity validation |
| `src/codegen/rust/typed_lowering.rs` | Rust thin wrapper over shared coercion (legacy bridge being removed) |

## Backend Consumption

| Backend | IR integration | Coercion path |
|---------|----------------|---------------|
| **Rust** | Full (production) | `encode_call_argument(..., Target::Rust)` |
| **Go** | Analyzer + IrPipeline | Go mutex/ref wrappers per encoding |
| **JavaScript** | Analyzer + IrPipeline | Pass-through + Readonly hints in `.d.ts` |
| **WASM** | Via Rust CodeGenerator | Linear memory pointer semantics |
| **WGSL** | Out of scope | GPU types only, no ownership |
| **Interpreter** | N/A (runtime refs) | Must pass cross-backend conformance |

Go and JavaScript backends run the same Analyzer + IrPipeline as Rust (parallel migration requirement).

## Incremental Cutover

`IrCutoverConfig` (`src/codegen/rust/generator.rs`) controls which codegen categories read from IR:

| Flag | Category | Env disable |
|------|----------|-------------|
| `ownership` | Formal param ownership | `WJ_IR_CUTOVER_DISABLE_OWNERSHIP=1` |
| `clones` | Auto-clone annotations | `WJ_IR_CUTOVER_DISABLE_CLONES=1` |
| `param_types` | Formal param types | `WJ_IR_CUTOVER_DISABLE_PARAM_TYPES=1` |
| `str_ref` | `&str` optimization params | `WJ_IR_CUTOVER_DISABLE_STR_REF=1` |
| `call_sites` | Call-site argument coercions | `WJ_IR_CUTOVER_DISABLE_CALL_SITES=1` |
| `locals` | Local variable types | `WJ_IR_CUTOVER_DISABLE_LOCALS=1` |

All production flags default **on** via `IrCutoverConfig::from_env()` (disable individually with the env vars above). The `Default` impl keeps flags **off** for unit tests that construct a bare `CodeGenerator` without env cutover.

## Shadow Validation

Compare IR solver results against legacy analyzer decisions before deleting heuristics:

```bash
wj build --ir-shadow-validate   # fail on discrepancies
```

Implementation: `src/ir/shadow.rs` — `validate_shadow(analyzed, registry)`.

Integration tests: `tests/ir_shadow_validation_test.rs`.

## TDD Protocol

For every coercion rule or constraint change:

1. Failing unit test in `src/ir/` (constraint or coercion)
2. Failing integration test in `tests/typed_lowering_test.rs` or `tests/ir_shadow_validation_test.rs`
3. Minimal implementation
4. Full suite: `cargo test --release --lib` + `cargo test --release --test all`
5. Cross-backend: `cargo test --release --test all --features conformance_tests`

**Banned:** New `matches!(method, ...)` lists for ownership decisions. Use signature-driven constraints instead (see `.cursor/rules/no-hardcoded-method-names.mdc`).

## Migration Phases

| Phase | Deliverable |
|-------|-------------|
| 0 | Shadow validation wired, cutover flags extended, integration tests |
| 1 | Expression IR, call-site constraints, solver write-back, analyzer on all backends |
| 2 | Shared `coercion.rs` + `target_encodings` emit |
| 3 | Parallel backend cutover (Rust, Go, JS, WASM) |
| 4 | Multipass IR merge, dogfood + breach-protocol dogfooding |
| 5 | Delete legacy heuristics (`call_site_borrow`, `correct_legacy_output`) — **in progress** |

### Phase 5 progress (2026-08-11)

- **IR total for known callees:** `apply_ir_call_site_coercion` no longer returns `None` on ownership-collision deferral; `skip_on_ownership_collision` removed.
- **Formals from IR:** when `OwnedType` is definitive borrow, skip body-walk `keep_owned_contract`; prefer `ir_param_ownership_definitive` / `get_effective_param_type`.
- **Missing boundary signatures:** module-qualified callees without an exact registry key emit `compile_error!("missing boundary signature for …")` — no name-based ownership guesses.
- **Deleted `typed_lowering::correct_legacy_output`** and all call sites; deleted `function_call_generation` `!call_sites` legacy auto-borrow block; removed `Vec::new()` string-prefix borrow heuristic.
- **Field-access fallback early-returns on IR** (no post-IR legacy double-patch); Copy-aggregate owned formal checks DRY'd into `call_site_borrow::{bare_type,sig_formal}_is_copy_aggregate_owned`.
- **Owned string formals:** `payload_forces_owned` beats stale IR Borrowed unless `str_ref_params` hints; `call_site_param_expects_owned_string` respects runtime/`&str` skips (`strings::starts_with`).
- **Post-IR DRY:** `reconcile_post_ir_mut_borrow_and_owned_peel` consolidates mut-borrow + owned/recursive peel; multi-candidate owned peel lives in `peel_stale_borrow_for_multi_candidate_owned_formal`.
- **IR-None fail-closed:** when `call_sites` is on and IR returns `None`, emit prepared arg and return — never fall through to legacy ownership tails.
- **Collection-key finalize in IR:** `finalize_ir_collection_key_arg` (binding-aware strip + `finalize_collection_key_call_site_arg`) runs at end of `apply_ir_call_site_coercion` and again at end of reconcile; method post-IR early/late collection-key clusters deleted.
- **String/text finalize in reconcile:** prefer_shared text-ref upgrade + `finalize_borrowed_text_call_site_arg` + FieldAccess `&str` ensure + string-literal finalize live in `reconcile_post_ir_mut_borrow_and_owned_peel`; regular_call duplicated text_sig block removed.
- **Post-IR tail collapse (2026-08-11):** deleted regular_call owned-literal / runtime-std / FieldAccess text-borrow clusters; moved vec-local borrow + `&"lit".to_string()` peel into reconcile; regular_call IR path uses a single terminal reconcile; method IR path runs reconcile last (covers builder bare-lit → owned String).
- **Cross-crate bare lit auto-own:** type-qualified / unresolved-instance helpers default to owned when no Pattern/`&str` evidence (no `::new` name lists); empty stub sigs fall through to the same path.
- **Prefer-shared enforce + copy-aggregate `&mut` peel in reconcile:** `enforce_ir_ownership_preserving_confirmed_shared_ref` and `peel_spurious_mut_borrow_on_owned_copy_aggregate` run at the start of `reconcile_post_ir_mut_borrow_and_owned_peel`; duplicated regular_call clusters removed; method IR mid-path enforce deleted (terminal reconcile owns the contract).
- **Method/field-access post-IR tails in reconcile:** `peel_copy_aggregate_caller_into_owned_callee` (regression-060), match-arm readonly text borrow, Pattern/`&str` normalize, runtime-std WJ-owned/Rust-borrowed (`json::get` via signature, not module name), stub associated/instance auto-own, shared-ref strip. Field-access IR paths now call terminal reconcile. Debug collision logs no longer filter hardcoded callee names.
- **Gates:** `tests/ir_call_site_total_coercion_test.rs`, `tests/ir_formal_param_emission_test.rs`, `tests/phase5_no_legacy_bridge_test.rs`, `tests/codegen_env_get_str_literal_must_not_auto_own_gate_test.rs`, `tests/codegen_starts_with_str_literal_must_not_auto_own_gate_test.rs`, `tests/codegen_cross_crate_associated_new_bare_literal_must_auto_own_gate_test.rs`.

Remaining: delete legacy `!call_sites` paths when opt-out is retired; fold remaining method mixed-forwarder / wants_ref clusters into IR coercion.

## Related Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — compiler crate structure
- [design/auto-reference.md](design/auto-reference.md) — historical auto-borrow design (being superseded)
- [design/multi-target-codegen.md](design/multi-target-codegen.md) — multi-target vision
- [proposals/rfcs/WJ-SEC-01-effect-capabilities.md](proposals/rfcs/WJ-SEC-01-effect-capabilities.md) — effect system
- [proposals/rfcs/WJ-SEC-02-taint-tracking.md](proposals/rfcs/WJ-SEC-02-taint-tracking.md) — taint tracking
- [proposals/rfcs/WJ-CONC-01-async-concurrency.md](proposals/rfcs/WJ-CONC-01-async-concurrency.md) — execution modes

## Success Criteria

- Zero heuristic method-name lists for ownership/coercion
- All backends run `IrPipeline` and use `encode_call_argument`
- `cargo test --release --test all` green; conformance 26/26 green
- dogfood + breach-protocol ownership E0308 errors eliminated
- Legacy `call_site_borrow` / `correct_legacy_output` deleted
