# Rust Leakage Audit: Session 2026-03-14

## Scope

Audited all code in `windjammer-game/windjammer-game-core` for Rust leakage patterns and Windjammer philosophy adherence. This audit follows the parallel TDD session that fixed 382 compilation errors.

**Audit Targets:**
- All `.wj` files in `src_wj/`, `examples/`, `tests/`, `tests_wj/`
- Compiler improvements (generic type propagation, trait ownership, mutation detection)

---

## Executive Summary

**Philosophy Compliance: FAIL**

**Critical Finding:** Extensive Rust leakage exists across the windjammer-game codebase. While the compiler improvements (generic type propagation, trait ownership inference, extended mutation detection) align perfectly with Windjammer philosophy, the **source code has not been updated** to use idiomatic Windjammer patterns.

The compiler is ready for idiomatic code. The source code is not.

---

## Findings

### P0: Explicit Ownership Annotations (CRITICAL)

**Status: FAIL** — Hundreds of violations found

#### `&self` / `&mut self` in Method Signatures

**Files with explicit ownership (sample):**

| File | Violations | Example |
|------|-------------|---------|
| `particles/emitter.wj` | 14 | `pub fn x(&self) -> f32` |
| `rendering3d/camera3d.wj` | 12+ | `pub fn fov(&self) -> f32` |
| `ecs/systems.wj` | 6 | `fn name(&self) -> string` |
| `editor/undo_redo.wj` | 9 | `fn execute(&self)` |
| `quest/quest.wj` | 6 | `pub fn id(&self) -> &QuestId` |
| `dialogue/system.wj` | 8+ | `pub fn next_node_id(&self) -> &DialogueLineId` |
| `animation/blend_tree.wj` | 14+ | `pub fn get_node_mut(&mut self, id: u32) -> Option<&mut BlendNode>` |
| `event/event.wj` | 12+ | `pub fn get_data_string(self, key: &str) -> Option<&str>` |
| `save/data.wj` | 12+ | `pub fn player_name(self) -> &str` |
| `state_machine/state.wj` | 14+ | `pub fn name(self) -> &str` |
| `dialogue/node.wj` | 8 | `pub fn id(self) -> &str` |
| `dialogue/choice.wj` | 6 | `pub fn text(self) -> &str` |
| `ecs/world.wj` | 2 | `pub fn query_transforms(self) -> &Vec<Entity>` |
| `examples/ecs_test.wj` | 20+ | `fn component_name(c: &Component) -> &str` |
| ... | ... | ... |

**Total files affected:** 80+ files  
**Total violations:** 300+ method signatures

**Correct form:**
```windjammer
// ❌ CURRENT (Rust leakage)
pub fn x(&self) -> f32 { self.pos_x }
pub fn emit(&mut self) { ... }

// ✅ IDIOMATIC WINDJAMMER
pub fn x(self) -> f32 { self.pos_x }      // Compiler infers &self
pub fn emit(self) { ... }                   // Compiler infers &mut self
```

#### Explicit `&` in Parameters and Return Types

**Examples found:**
- `fn process_color(color: &Color) -> Color` (post_processing.wj)
- `pub fn astar_smooth_path(path: AStarPathResult, grid: &AStarGrid)` (astar_grid.wj)
- `pub fn encode_region(self, grid: &VoxelGrid, ...)` (svo.wj)
- `pub fn from_grid(grid: &VoxelGrid) -> Octree` (octree.wj)
- `pub fn has_tag(self, tag: &str) -> bool` (query_system.wj)
- `pub fn find_by_tag(self, tag: &str) -> Vec<&Entity>` (query_system.wj)
- `pub fn start(self, node_id: &str)` (dialogue/manager.wj)
- `fn extract_extension(path: &string) -> string` (asset_browser.wj)
- `fn apply_brush_erase(self, op: &mut EditOperation)` (voxel_editor.wj)

**Correct form:** Remove all `&` and `&mut` — compiler infers from usage.

#### Explicit `&` in Function Calls (Map/Collection Access)

**Examples:**
- `came_from.get(&node)` → should be `came_from.get(node)`
- `self.chunks.contains_key(&chunk_pos)` → should be `self.chunks.contains_key(chunk_pos)`
- `self.chunks.get_mut(&chunk_pos)` → should be `self.chunks.get_mut(chunk_pos)`
- `count_active(&entities)` → should be `count_active(entities)`

---

### P1: Rust-Specific Methods (HIGH)

**Status: FAIL** — 120+ violations found

#### `.unwrap()` — 100+ occurrences

**Heaviest offenders:**

| File | Count | Context |
|------|-------|---------|
| `scene_graph/scene_graph_state.wj` | 45 | Map access, node traversal |
| `editor/hierarchy_panel.wj` | 8 | Parent/child lookups |
| `assets/asset_manager.wj` | 8 | Asset retrieval |
| `tests_wj/*_conversion_test.wj` | 50+ | Assertions on Option/Result |
| `rendering/bvh.wj` | 4 | Hit result unwrapping |
| `editor/undo_redo.wj` | 2 | Stack pop |
| `ecs/entity.wj` | 1 | Free list pop |
| `voxel/octree.wj` | 2 | Children access |
| `csg/evaluator.wj` | 1 | Node access |
| `csg/scene.wj` | 1 | Node access |

**Correct form:**
```windjammer
// ❌ CURRENT (Rust leakage)
let node = self.nodes.get(id).unwrap()

// ✅ IDIOMATIC WINDJAMMER
if let Some(node) = self.nodes.get(id) {
    // use node
}
// or
match self.nodes.get(id) {
    Some(node) => { /* use node */ }
    None => { /* handle */ }
}
```

**Note:** In test files, `unwrap()` in assertions like `assert_eq!(x.unwrap(), expected)` could use `assert_is_some()` + `if let` pattern, or a test helper.

#### `.iter()` — 20+ occurrences

**Files:**
- `ecs/query_system.wj` (10) — `self.entities.iter()`, `self.tags.iter()`
- `event/dispatcher.wj` (6) — `self.event_log.iter()`
- `ecs/entity.wj` (2) — `self.components.iter()`, `required.iter()`
- `ecs/query.wj` (2) — `self.required.iter()`, `entity_components.iter()`
- `editor/scene_editor.wj` (2) — `self.selected_entities.iter()`, `self.clipboard.iter()`
- `physics/physics_world.wj` (1) — `collisions.iter()`
- `ui/text_input.wj` (4) — `chars.iter().collect()`, `selected.iter().collect()`
- `tests_wj/ecs_components_test.wj` (1) — `array.iter()`

**Correct form:**
```windjammer
// ❌ CURRENT (Rust leakage)
for item in self.entities.iter() { ... }
let names = self.entities.iter().map(|e| e.name.clone()).collect()

// ✅ IDIOMATIC WINDJAMMER
for item in self.entities { ... }
let names = self.entities.map(|e| e.name.clone())  // or direct iteration
```

**Note:** Some `.iter()` usages are for `.filter()`, `.map()`, `.any()` — Windjammer may need high-level collection methods. Direct `for item in collection` works; functional style may need stdlib support.

#### `.as_bytes()` — 4 occurrences

**Files:**
- `tests/rendering_guardrails_test.wj:208` — `data.as_bytes()`
- `tests_wj/quest_test.wj:336` — `quest.id.as_bytes()`
- `tests_wj/dialog_test.wj:375` — `node.id.as_bytes()`
- `tests_wj/dialog_simple_test.wj:124` — `node.id.as_bytes()`

**Correct form:** Use a backend-agnostic API (e.g., `string_to_bytes(s)` or similar) or pattern match. `.as_bytes()` is Rust-specific.

---

### P2: Reference Types and Method Names (MEDIUM)

**Status: PARTIAL**

#### `TranslationKey::as_str()` (localization/translation_key.wj)

```windjammer
pub fn as_str(&self) -> string {
    self.key.clone()
}
```

- **Issue:** Method name `as_str` is Rust-like; signature uses `&self`.
- **Recommendation:** Rename to `key()` or `to_string()` / `value()`, and use `self` instead of `&self`.

#### `.to_string()` Usage

**Status:** Conditionally acceptable per no-rust-leakage rule: "only if explicit copy needed."

- String literals: `"idle".to_string()` — common, often needed for `String` construction.
- Numeric: `count.to_string()` — explicit conversion.
- **Recommendation:** Keep where an owned `String` is required; avoid when compiler could infer.

---

### Exception: Trait Implementations Matching Rust Stdlib

**Rule:** "Trait method implementations can have explicit self types when matching stdlib traits."

**Checked:**
- `editor/undo_redo.wj` — `Command` trait with `fn execute(&self)`, `fn undo(&self)` — **custom trait**, not stdlib. Should use `self`.
- `physics/jolt/world.wj` — `fn drop(&mut self)` — **Drop** is Rust stdlib. **Exception applies** — explicit `&mut self` is correct here.

---

## Compiler Improvements: Philosophy Alignment

### Generic Type Propagation ✅

**Question:** Does it reduce boilerplate?  
**Answer:** YES. Developer writes `fn identity<T>(x: T) -> T { x }`; compiler propagates `<T>` through decorators and wrappers. No manual type parameter management.

**Alignment:** PERFECT ✅

### Trait Implementation Ownership ✅

**Question:** Does it reduce explicit annotations?  
**Answer:** YES. Developer writes `impl Trait for Type { fn method(self) { /* mutates */ } }`; compiler infers `&mut self` when matching trait. Trait impls use trait's ownership.

**Alignment:** PERFECT ✅

### Extended Mutation Detection ✅

**Question:** Does it catch common patterns?  
**Answer:** YES. Detects `.take()`, `.push()`, `.insert()`, etc. ~90% of mutations inferred automatically.

**Alignment:** PERFECT ✅

**Verdict:** All three compiler improvements align with Windjammer values:
- "Infer what doesn't matter" ✅
- "Explicit where it does" ✅
- "80% of Rust's power, 20% of complexity" ✅

---

## Violations by Category (Summary)

| Category | Count | Severity | Fix Effort |
|----------|-------|----------|------------|
| `&self` / `&mut self` in methods | 300+ | P0 Critical | High |
| `&` in params/returns | 80+ | P0 Critical | Medium |
| `.unwrap()` | 100+ | P1 High | Medium |
| `.iter()` | 20+ | P1 High | Low–Medium |
| `.as_bytes()` | 4 | P1 High | Low |
| `&` in function calls | 30+ | P0 Critical | Low |

---

## Recommendations

### Immediate (P0)

1. **Remove all `&self` and `&mut self`** from method signatures. Use `self`; let the compiler infer.
2. **Remove all `&` and `&mut`** from parameter and return types.
3. **Remove `&` from function calls** (e.g., `get(&key)` → `get(key)`).

### High Priority (P1)

4. **Replace `.unwrap()`** with `if let Some(x) = ...` or `match` in production code.
5. **Replace `.iter()`** with direct iteration: `for item in collection`.
6. **Replace `.as_bytes()`** with a backend-agnostic API or equivalent.

### Medium Priority (P2)

7. **Rename `TranslationKey::as_str()`** to `key()` or `value()`.
8. **Review `.to_string()`** usage; keep only where an owned `String` is required.

### Phased Approach

Given 80+ files and 500+ violations:

**Phase 1:** Core engine (`ecs/`, `rendering/`, `physics/`, `animation/`)  
**Phase 2:** Game systems (`dialogue/`, `quest/`, `event/`, `save/`)  
**Phase 3:** Editor, UI, tests, examples  

---

## Files Requiring Most Attention

**Top 15 by violation density:**

1. `scene_graph/scene_graph_state.wj` — 45+ unwraps, ownership
2. `ecs/query_system.wj` — iter, ownership, refs
3. `dialogue/system.wj` — ownership, ref returns
4. `animation/blend_tree.wj` — `Option<&mut T>`, ownership
5. `event/event.wj` — `&str` params/returns
6. `save/data.wj` — `&str` params/returns
7. `state_machine/state.wj` — `&str` params/returns
8. `dialogue/node.wj` — `&str` returns
9. `dialogue/choice.wj` — `&str` params/returns
10. `editor/hierarchy_panel.wj` — unwraps, ownership
11. `assets/asset_manager.wj` — unwraps
12. `examples/ecs_test.wj` — heavy Rust-style (demo)
13. `editor/undo_redo.wj` — trait with `&self`
14. `ecs/world.wj` — `&Vec<Entity>` return
15. `ai/astar_grid.wj` — `&AStarGrid` param

---

## Success Criteria (Current Status)

| Criterion | Status |
|-----------|--------|
| Zero Rust leakage in .wj files | ❌ FAIL |
| All improvements philosophy-aligned | ✅ PASS |
| Idiomatic Windjammer code | ❌ FAIL |

---

## Final Verdict

**Philosophy Compliance: FAIL**

**Summary:**
- **Compiler:** Philosophy-aligned. Generic propagation, trait ownership, and mutation detection support idiomatic Windjammer.
- **Source code:** Significant Rust leakage. 80+ files, 500+ violations across P0 and P1.

**Action Required:** Systematic refactor of windjammer-game-core to remove Rust leakage. Prioritize by module (core engine first), then game systems, then editor/tests.

**Remember:** "If you're typing `.as_str()`, `.as_ref()`, or `&mut`, you're writing Rust, not Windjammer."

---

*Audit completed: 2026-03-14*  
*Auditor: Rust Leakage Auditor (no-rust-leakage.mdc)*
