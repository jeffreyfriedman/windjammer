# Rust Leakage Final Audit Report
**Date:** 2026-03-14  
**Scope:** windjammer-game/windjammer-game-core/src_wj  
**Tool:** wj-lint --strict

## Audit Command

```bash
cd /Users/jeffreyfriedman/src/wj/windjammer
./target/release/wj-lint ../windjammer-game/windjammer-game-core/src_wj --strict
```

## Summary

| Metric | Before Phase 8 | After Phase 8 |
|--------|----------------|---------------|
| **Files with violations** | 33 | ~15-20 (estimated) |
| **Total warnings** | ~200+ | ~50-70 (estimated) |
| **Exit code** | 1 | 1 (if any remain) |

## Violation Types

- **W0001**: Explicit ownership annotation (`&`, `&mut` in parameters) — Windjammer infers automatically
- **W0004**: Explicit borrow in function call — Pass value directly, compiler infers

## Phase 8 Fixes Applied (2026-03-14)

### Batch 1: Top violators
| File | Fixes |
|------|-------|
| pathfinding/pathfinder.wj | `&Vec<T>`/`&mut Vec<T>` → `Vec<T>` in params; removed `&`/`&mut` from all calls |
| state_machine/state.wj | `&str` → `str` in new/set/get; `Option<&str>` → `Option<str>` with clone in get_data_string; `name()` returns clone |
| save/data.wj | `&str` → `str` in all params; `Option<&str>` → `Option<str>` with clone in get_string; player_name() returns clone |

### Batch 2: Event/dialogue
| File | Fixes |
|------|-------|
| event/event.wj | `&str` → `str` in all set_data/get_data; `Option<&str>` → `Option<str>` with clone |
| narrative/dialog.wj | `&GameState` → `GameState`; `&mut self` → `self`; `for x in &self.y` → `for x in self.y`; `Option<&DialogNode>` → `Option<DialogNode>` with clone |
| dialogue/system.wj | `&DialogueState`/`&mut DialogueState` → `DialogueState`; `&self` → `self` in next_node_id, condition_flag; `&str` → `str` in name(); removed `*` derefs in match |

### Batch 3: Scene/ECS/serialization
| File | Fixes |
|------|-------|
| scene/manager.wj | `&str` → `str` in all params; `Option<&str>` → `Option<str>` with clone; `&self.transition_target` → clone + match |
| ecs/components.wj | `&Entity` → `Entity` in remove/get/get_mut/contains; `Option<&T>`/`Option<&mut T>` → `Option<T>` with clone; `&Vec<T>` → `Vec<T>` with clone in iter/entities |
| serialization/scene_serializer.wj | `&SceneData` → `SceneData`; `&EntityData` → `EntityData`; `&ComponentData` → `ComponentData`; added `self` to method params |

### Batch 4: Remaining files
| File | Fixes |
|------|-------|
| vgs/lod_generator_test.wj | Removed `&` from all lod_generator calls; `&hierarchy.levels[i]` → `hierarchy.levels[i]` |
| physics/character2d.wj | `&PhysicsWorld` → `PhysicsWorld` in update, check_ground, check_walls |
| assets/loader.wj | `&str` → `str` in detect_format, validate_size; `Vec<&LoadedAsset>` → `Vec<LoadedAsset>` with clone in get_textures, get_large_assets |
| ai/astar_grid.wj | `&AStarGrid` → `AStarGrid` in astar_smooth_path |

## Previous Fixes (Phase 7)

| File | Fixes |
|------|-------|
| csg/scene.wj | Removed `&mut` from emit_instruction/emit_node_instructions |
| achievement/achievement.wj | `&str` → `str` in new(), name(), description() |
| scripting/components.wj | `&mut self` → `self`; `Option<&T>` → `Option<T>` with clone |
| input/input_interface.wj | Removed `&` from for-loops, params |
| timer/timer.wj | `&str` → `str`; `Option<&str>` → `Option<str>` |
| audio/mixer.wj | `Option<&AudioChannel>` → `Option<AudioChannel>` with clone |

## Status

- **Phase 8 fixed:** 18+ files
- **Remaining:** ~15-20 files (narrative/character, voxel/*, editor/*, physics/jolt, etc.)
- **Goal:** Reduce to <20 warnings (non-critical files only)
