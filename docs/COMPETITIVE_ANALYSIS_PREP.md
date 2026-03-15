# Competitive Analysis Prep

**Purpose**: Roadmap data for benchmarking Windjammer vs. major game engines.  
**Source**: Synthesized from HANDOFF_2026_03_15, COMPREHENSIVE_STATUS_AND_ROADMAP, GAME_ENGINE_IMPROVEMENTS_DESIGN, FINAL_SESSION_SUMMARY.

---

## Target Engines

| Engine | Role | Notes |
|--------|------|-------|
| **Unity** | Industry standard | ShaderGraph, C#, large ecosystem |
| **Unreal** | AAA reference | Nanite, Material Editor, Blueprints |
| **Godot** | Open-source | VisualShader, GDScript, lightweight |
| **Bevy** | Rust ecosystem | ECS, shader pipeline, data-driven |

---

## Feature Comparison Matrix

### Rendering

| Feature | Unity | Unreal | Godot | Bevy | Windjammer |
|---------|-------|--------|-------|------|------------|
| Virtual geometry (VGS/Nanite) | ❌ | ✅ Nanite | ❌ | ❌ | ✅ VGS |
| Shader validation | Compile-time | Compile-time | Compile-time | Compile-time | Runtime only |
| Frame debugger | Built-in | Built-in | Basic | Limited | Manual PNG |
| Visual profiler | Built-in | Built-in | Basic | Limited | None |
| G-buffer pipeline | ✅ | ✅ | ✅ | ✅ | ✅ |

### Developer Experience

| Feature | Unity | Unreal | Godot | Bevy | Windjammer |
|---------|-------|--------|-------|------|------------|
| Hot reload | Scripts | Blueprints | Scripts | Limited | Full rebuild |
| Error messages | Context-aware | Blueprint-friendly | Basic | Rust | Rust panic / wgpu raw |
| Type safety | ⚠️ Weak | ✅ Strong | ⚠️ Weak | ✅ Strong | ✅ Strong |
| Memory safety | ⚠️ GC | ❌ Manual | ⚠️ RC | ✅ Ownership | ✅ Ownership |

### Systems

| Feature | Unity | Unreal | Godot | Bevy | Windjammer |
|---------|-------|--------|-------|------|------------|
| ECS | DOTS (optional) | Optional | ❌ | ✅ Core | SceneBuilder |
| Plugin system | Packages | Plugins | GDExtension | Plugins | Hybrid |
| Scene setup | Visual | Visual | Visual | Code | Code (SceneBuilder) |

---

## Performance Metrics to Track

### Rendering

- **Frame time** (ms) – target: 16.67ms (60 FPS)
- **VGS visibility pass** – clusters/second, ms per pass
- **VGS expansion pass** – triangles/second, atomic append throughput
- **Voxel raymarch** – rays/second, SVO traversal cost
- **Memory usage** – VRAM, buffer sizes

### Compilation

- **Full game build** – seconds (current: ~4s)
- **Shader iteration** – seconds (target: <1 min with .wjsl)

### Load Time

- **Scene load** – seconds
- **Asset import** – seconds

---

## Developer Experience Criteria

- **Shader ergonomics**: Compile-time validation vs. runtime colored stripes
- **Error clarity**: Line numbers, context, suggested fixes
- **Iteration speed**: Edit → see result (target: <1 min)
- **TDD support**: Test shaders like regular code
- **Documentation**: API docs, examples, tutorials

---

## Success Criteria: What Does "Competitive" Mean?

### Tier 1: Baseline

- [ ] Feature parity documented (feature matrix complete)
- [ ] Performance measured (benchmarks run)
- [ ] Gaps documented with priorities

### Tier 2: Competitive

- [ ] Within 2x of Unity/Unreal/Godot/Bevy on primary metrics
- [ ] Clear strengths (e.g., memory safety, ownership, TDD)
- [ ] Roadmap for top 3 gaps

### Tier 3: Leading

- [ ] Within 1.5x of best-in-class on all metrics
- [ ] Clear differentiators (e.g., shader ergonomics, safety)
- [ ] Marketing materials ready

---

## Key Questions to Answer

1. **Rendering**: How does VGS compare to Nanite?
2. **Shaders**: How does shader ergonomics compare to ShaderGraph?
3. **ECS**: How does our system compare to Bevy?
4. **Physics**: Jolt vs. Unity/Unreal physics?
5. **Scripting**: Windjammer vs. C#/Blueprint/GDScript?

---

## Implementation Order

1. **Week 1**: Benchmarking – run rendering, compilation, memory tests
2. **Week 2**: Feature analysis – document parity, gaps, priorities
3. **Week 3**: Implementation – fill top 3 gaps, update docs
