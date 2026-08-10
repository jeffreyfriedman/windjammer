# IR-Total Call-Site Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make IR coercion the sole path for known callees, emit formals from IR param types, and hard-error on missing boundary signatures (no name-based ownership guesses).

**Architecture:** Extend `IrCutoverConfig` + `apply_ir_call_site_coercion` so known callees never fall through to legacy. Formal emission trusts `IrFunction.param_types` over body-walk keep-owned. Boundary calls without registry metadata fail closed.

**Tech Stack:** Windjammer compiler (`src/codegen/rust`, `src/ir`), Rust integration tests under `tests/`.

## Global Constraints

- No hardcoded method/function name lists for ownership/coercion
- Maximize IR/solver; minimize heuristics
- TDD: failing gate before each behavioral change
- Prefer DRY/SOLID factoring over one-off patches

---

## Task 1: IR total — delete collision → legacy bailout

**Files:**
- Modify: `src/codegen/rust/ir_call_site.rs`
- Modify: call sites of `apply_ir_call_site_coercion` (drop `skip_on_ownership_collision`)
- Test: `tests/ir_call_site_total_coercion_test.rs`

- [ ] Write failing test: known callee with ownership collision still gets IR `&`/`owned` (never bare legacy)
- [ ] Remove `return None` collision block in `apply_ir_call_site_coercion`
- [ ] Remove `skip_on_ownership_collision` parameter
- [ ] Run gate + related ownership regressions

## Task 2: Missing signatures = hard errors at boundaries

**Files:**
- Modify: `src/codegen/rust/generator.rs` (error accumulator)
- Modify: `src/codegen/rust/program_generation.rs` (flush errors)
- Modify: `src/codegen/rust/ir_call_site.rs` (no-sig boundary path)
- Modify: `src/codegen/rust/string_utilities.rs` (stop conservative `true` guesses)
- Test: `tests/ir_boundary_missing_signature_test.rs`

- [ ] Write failing test: module-qualified call with no meta → compile error message
- [ ] Accumulate `missing boundary signature for …` errors; fail `generate_program`
- [ ] Change unknown associated/instance literal helpers to return `false` (no guess)
- [ ] Update unit tests that expected conservative owned

## Task 3: Formals from IR param types only

**Files:**
- Modify: `src/codegen/rust/generator.rs` (`get_effective_param_type`, demotion)
- Modify: `src/codegen/rust/function_formal_parameter_generation.rs`
- Test: `tests/ir_formal_param_emission_test.rs`

- [ ] Write failing test: IR Borrowed string formal emits `&str` without keep-owned body walk winning
- [ ] Prefer IR ownership when `OwnedType` is not `Inferred`; skip `keep_owned_contract` override
- [ ] Route type selection through `get_effective_param_type`

## Task 4: Delete dead legacy call-site phases (when call_sites on)

**Files:**
- Modify: `function_call_generation.rs`, `arguments.rs`, `regular_call_arguments.rs`, `field_access_method_args.rs`
- Docs: `docs/IR_SOLVER_CODEGEN_ARCHITECTURE.md`

- [ ] Assert/unreachable if IR returns `None` for known callee with call_sites on
- [ ] Remove post-IR duplicate legacy borrow where safe
- [ ] Document Phase 5 progress

## Task 5: Verify + land

- [ ] `unset CARGO_TARGET_DIR && cargo test --release --test all --features skip_fixtures -- ir_`
- [ ] Broader ownership/string gates
- [ ] Commit + push when user requests (or per AGENTS.md session end)
