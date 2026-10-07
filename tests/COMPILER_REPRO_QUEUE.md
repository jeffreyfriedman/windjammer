# Compiler repro queue (dogfooding — do not work around in application code)


## P3.719 (2026-10-07) — TDD WDB-463 (DB agent; no compiler src)

Owned `Vec<Copy>` for-loop must not demote to `&` + `.clone()` into `Vec::push`.

Product `scene_graph/scene_graph_state.rs` `remove_node`:
```wj
let children_copy = node.children
for child_id in children_copy {
    to_remove.push(child_id)
}
```
Tip MultiFile keeps owned for + bare `push(child_id)` (isolate GREEN). Tip-out
emits `for child_id in &children_copy` + `to_remove.push(child_id.clone())`.

| Gate | Status |
|------|--------|
| WDB-463 MultiFile | ✅ isolate GREEN — owned for + bare push |
| WDB-463 tip-out | ❌ tip RED — `&children_copy` + `child_id.clone()` in `rel_tip_out/scene_graph/scene_graph_state.rs` |

**Root cause layer:** tip-out / product multipass lag — isolate tip already correct;
owned `Vec<u64>` for-loops still demoted to borrowed + Copy `.clone()` into push.

**Why this is a new class:**
- WDB-462 is Copy newtype from **HashMap.keys()** (inherently borrowed).
- WDB-457 is owned Copy **newtype local** into push.
- WDB-431 is Copy u64 **field** into insert/push.
- This is owned `Vec<Copy>` for-loop demoted to `&` + `.clone()` into push.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb463_` — isolate GREEN / tip RED (2026-10-07).

**Do not steal:** WDB-406/408/411/457–463, P3.508–P3.719, WDB-412–463 (filed).

## P3.720 (2026-10-07) — directory package must emit `mod.rs` + re-exports into `--output`

Breach Protocol `src/inventory/` (directory module) tip-transpile to `gen/` writes
`gen/inventory/{item,item_id,…}.rs` but **omits** `gen/inventory/mod.rs`.
Root `lib.rs` then hits E0583; thin synthesized decls miss `pub use ItemId`
(cascading E0425). wj-game restores from stale `build/` as a host workaround.

| Gate | Status |
|------|--------|
| `directory_module_must_emit_mod_rs_with_reexports` | ✅ tip GREEN (MultiFile + CLI `--output gen`) |
| product tip-out lag / wiped gen | wj-game `ensure_gen_directory_mod_rs` host guardrail |

**Root cause layer:** multipass / `--module-file` emit for directory packages into
`--output gen` must write `gen/<pkg>/mod.rs` with child `pub mod` + public re-exports.

**Do not steal:** wj-game restore-from-`build/`; P3.718–P3.719 (other agents).

**Gates:** `cargo test --release --test all --features integration_tests -- directory_module_must_emit_mod_rs_with_reexports`.

#