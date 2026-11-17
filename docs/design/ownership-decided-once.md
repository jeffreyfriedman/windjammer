# Ownership decided once

## PC verification commands (wt-ownership-16 b9fbeb2c2, then wt-ownership-17)

Run in a checkout of the branch, `CARGO_BUILD_JOBS=2`, through the slot gate; `<all>` is the `--test all` binary.

1. `cargo build --release --bin wj && cargo test --release --lib --no-run && cargo test --release --test all --no-run`
2. `<all> codegen_correctness_test library_module_receiver_shapes std_ stdlib_scanner heuristic_surface architecture inference_spec_catalog drift_contract width_prediction_test prediction_parity_test explicit_ref_of_an_integer_literal`
3. `WJ_GATE_JOBS=2 <all> --test-threads=1 convergence_gates_test project_gates_test` (no UPDATE_GOLDEN; a changed golden
   is bisected against the parent commit first)
4. Gate 5 inventory, printed: `WJ_GATE_FILTER=. WJ_GATE_JOBS=2 <all> --test-threads=1 --exact convergence_gates_test::gate5_formals_emitted_against_the_ir_only_shrink --nocapture`;
   recorded once on ownership-16: the same with `UPDATE_GOLDEN=1` and without the filter, then commit
   `tests/golden/own_stages_emitted_vs_ir.txt`
5. `wj run --verify bench/agent-dx/solutions/wj-set3/t308_text_world/main.wj` (pilot-7 references 9/9)
6. Ecosystem: full builds of packages/wj-sql, apps/wj-webhook, packages/wj-duration (cargo check)
7. `wj build tests/engine_parity/jit_map_ops.wj --no-cargo -o <dir>` then `cargo check` in `<dir>`
8. Mutation: on the parent commit the new corpus cases fail (field_passed_to_owned_formal_keeps_borrowed_receiver,
   builder_reads_fields_after_storing_one, std_text_formal_borrows_owned_argument, borrowed_copy_key_into_map_lookup,
   library_module_receiver_shapes::a_method_does_not_take_the_formals_of_a_same_named_free_function)
9. Engine by code, parent and head: `WJ=<wj> scripts/build_engine.sh transpile` (game-host), then in
   game-host-core `cargo check --lib --message-format short > c.log 2>&1; grep -oE "error\[E[0-9]+\]" c.log | sort | uniq -c`
10. Full suite once: `<all> --test-threads=3`, compared with the parent's failure list
11. On wt-ownership-17 (class 1): steps 1-4 and 9; gate 5's printed list before/after shows the entries class 1 removes

Status: DRAFT (stage 0 landed as gate 5; stage 1 class 1 drafted, unbuilt, not for merge).

## Problem

A parameter's ownership (`&T`, `&mut T`, owned `T`) must be decided in one place: the solver's fixed point
(`src/ir/constraint_gen.rs`, `src/ir/solver.rs`, `src/compiler/value_formal_fixed_point.rs`). Every caller reads that
decision through the published signature. Today the Rust backend re-decides some formals while it emits them, after the
solver has finished, and publishes the emitted formal only when the callee's own file is generated. A caller generated
earlier reads the solved formal and the two disagree (rustc E0308 at the call). Two engine cases:

- `World::get_variable(self, key: string)` in `dialogue_system.wj` (a split `impl World`, the file imports the caller's
  module): solved `key: &str`, emitted `key: String`; the caller in `dialogue_tree.wj` passes `&String`.
- `ReverbZone::contains(self, x: f32, ..)`: solved owned, emitted `x: &f32` (a homonym's signature read by bare name;
  fixed separately by keying methods as `Type::method`).

`wj explain-ownership` reports each case as `D-OWN-STAGES ... ir=X emitted=Y`.

## Stage 0: inventory (landed)

`convergence_gates_test::gate5_formals_emitted_against_the_ir_only_shrink` lists every formal over the convergence
corpus whose emitted mode differs from the IR's mode (`tests/golden/own_stages_emitted_vs_ir.txt`, one line per
`file::function::param`). A new entry fails the gate; a fixed one is locked in with `UPDATE_GOLDEN=1`. The list is the
work queue of stage 1 and must reach zero.

## Rules the backend applies after the solver (text formals)

In `src/codegen/rust/function_formal_parameter_generation.rs` (line numbers at b9fbeb2c2):

| Rule | Where | What it re-decides |
|---|---|---|
| `pub_module_api_keeps_owned_string_formal` | :3831, used at :234, :348, :658, :685, :798, :840, :854, :920, :981, :1070, :1366, :1404, :1433 | owned `String` for `pub` text formals: unused formals of free functions (class 1), forwards into owned callees, readonly uses that still feed an owned payload |
| `param_should_emit_into_string_formal` | :3238, used at :354 | `impl Into<String>` builder/setter formals that store into the returned payload |
| `param_pub_free_string_builder_forward` | :3496 | owned text formals of `pub` free builders that forward |
| `str_ref_optimized_params` insertion | :226, :331, :535, :665, :715, :829, :843, :886, :942, :983, :1193, :1390, :1424; `function_generation_prepare.rs` :380, :395, :409 | `&str` demotion of readonly text formals |
| `into_string_formal_params` insertion | :360, :902, :992 | `impl Into<String>` spelling |
| `param_must_not_demote_to_shared_borrow` | :3947 | keeps owned where a borrow would be emitted |
| `formal_from_ir_owned` text exception | :3192 | text formals keep the rules above instead of the IR's owned mode |

The analyzer has two text rules that feed the solver's seeds (already before the solver, kept as seeds until the
solver owns them): `Analyzer::analyze_str_ref_optimizable_params` (`src/analyzer/string_optimization.rs:44`) and
`module_string_in_mutual_recursion_owned_contract` (`src/analyzer/passthrough_inference.rs:2197`).

## Stage 1: move each rule into the solver, class by class

Each class becomes a constraint rule in `src/ir/constraint_gen/text_formals.rs` with its own rule ID (cataloged in
`src/ir/explain/rules.rs` and `docs/spec/inference.md`), so the solved mode and the emitted mode agree. Per class, one
commit with:

1. the constraint rule;
2. a rustc-checked corpus case (multi-file where the class is about cross-file callers);
3. gate 5's inventory lowered by the entries the class covers (recorded on the PC with `UPDATE_GOLDEN=1`);
4. goldens unchanged except the intended ones. A changed golden is bisected first (the commit before and after the
   class, the single function whose formal moved, `wj explain-ownership` for it); it is re-recorded only when the new
   output is the class's intended formal and rustc accepts it.

Order (by how many engine/corpus entries each covers, smallest blast radius first):

1. OWN-PUB-TEXT-UNUSED: unused `string` formals of `pub` free functions are owned (drafted).
2. Methods' text formals, including split `impl` blocks (the `World::get_variable` case; its corpus case is
   `~/src/wj-wt/findings/ownership-15/split_impl_in_cycle_call_site_stale`, to be added with the class).
3. Forwards into owned callees (`passed_into_owned` in `pub_module_api_keeps_owned_string_formal`).
4. `impl Into<String>` builders (`param_should_emit_into_string_formal`, `param_pub_free_string_builder_forward`).
5. Readonly `&str` demotion (`str_ref_optimized_params`): the solver's OWN-STR-REF already decides it; the backend's
   insertions become reads of the solved mode.

## Stage 2: delete

Once a class has no gate-5 entry, its backend rule is deleted (not disabled): the backend spells the solved mode and
nothing else. When every class is gone, `formal_from_ir_owned`'s text exception and `ir_value_formal`'s "text formals
keep their own spelling rules" clause go too, and gate 5's inventory file is empty and stays so. No compatibility
switch: one decision.

## Interim safety

A caller reading a signature whose callee file has not been emitted yet is a stale read only while the backend still
re-decides formals. Once stage 2 is done there is nothing to read late. Until then the gate-5 inventory is the
measure.
