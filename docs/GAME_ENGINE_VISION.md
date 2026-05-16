# Windjammer game framework (vision)

The **Windjammer** repository is the compiler, stdlib, and language tooling.

The **`windjammer-game`** companion repository is the runtime-first game framework: ECS, shaders, renderer, plugins (`wj game`), and dogfood targets.

Canonical vision and competitor framing live in the **`windjammer-game`** companion repository (sibling checkout):

- **`windjammer-game/VISION.md`**
- **`windjammer-game/COMPETITIVE_POSITIONING.md`**

Open these from your multi-repo workspace root (`../windjammer-game/` when this repo sits next to it).

---

## Snapshot (marketing)

**Unified Renderer. Memory-Safe. Code-First Automatable. Deploy Anywhere.**

Windjammer is a **unified renderer** game engine that blends voxel and mesh pipelines so studios can pursue destructible voxel worlds **and** character-driven cinematic content from one stack. It is built on memory-safe foundations, emphasizes **code-first, TDD-driven** workflows (automatable playtesting and visual regressions—uncommon as first-class workflows in UE/Unity/Godot), and compiles via **Rust, Go, JavaScript**, and more for **desktop, WASM**, and **mobile** as targets mature.
