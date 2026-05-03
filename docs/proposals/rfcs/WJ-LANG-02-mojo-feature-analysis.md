# WJ-LANG-02: Mojo Language Feature Analysis

**Status**: Draft  
**Author**: Windjammer Team  
**Created**: 2026-03-28  
**Category**: Language Design

## Summary

This RFC analyzes features from the Mojo programming language that could be advantageous for Windjammer to adopt, modified to align with our core philosophy: **80% of Rust's power with 20% of Rust's complexity**, and **the compiler does the hard work, not the developer**.

Mojo is built on MLIR (Multi-Level Intermediate Representation), targets Python compatibility, and introduces several novel ideas around compile-time metaprogramming, SIMD abstractions, and value lifecycle management. While Windjammer has different goals (Rust interop, game development focus, multi-backend compilation), several of Mojo's design decisions are worth adopting or adapting.

## Mojo Feature Overview

### Features Analyzed

| Feature | Mojo Approach | Windjammer Relevance | Recommendation |
|---|---|---|---|
| Compile-time metaprogramming | `@parameter for/if`, alias, comptime execution | HIGH | Adopt (adapted) |
| SIMD abstractions | Native `SIMD[DType, N]` type | HIGH | Adopt for game/shader work |
| Value semantics by default | Copies are independent, ASAP destruction | MEDIUM | Already aligned |
| `@value` decorator | Auto-generates init/copy/move/del | HIGH | Already implemented (auto-derive) |
| Parameter system (generics) | Types AND values as compile-time params | MEDIUM | Expand existing generics |
| Autotuning | Removed from language, now library | LOW | Skip (complexity vs value) |
| MLIR backend | Multi-level IR for hardware targeting | LOW | Not applicable (Rust/Go/JS backends) |
| Python interop | Python superset | NONE | Not relevant |

## Recommended Adoptions

### 1. Compile-Time Metaprogramming (Priority: HIGH)

**Mojo's approach**: `@parameter for` unrolls loops at compile time, `@parameter if` evaluates conditions at compile time, and arbitrary code can run during compilation to compute parameter values.

```python
# Mojo compile-time loop unrolling
@parameter
for i in range(4):
    process(data[i])  # Generates 4 separate calls

# Mojo compile-time conditional
@parameter
if target == "gpu":
    use_gpu_kernel()
else:
    use_cpu_fallback()
```

**Windjammer adaptation**: Use `comptime` blocks and decorators that align with Windjammer syntax:

```windjammer
// Compile-time loop unrolling (zero-cost at runtime)
comptime for i in 0..4 {
    process(data[i])
}

// Compile-time conditional (dead code eliminated)
comptime if cfg("target_gpu") {
    use_gpu_kernel()
} else {
    use_cpu_fallback()
}

// Compile-time constants with computation
comptime {
    let LOOKUP_TABLE = generate_sin_table(1024)
}
```

**Why this matters for Windjammer**:
- Game engines need compile-time loop unrolling for performance-critical inner loops
- Shader variant generation benefits from compile-time conditionals
- Look-up table generation at compile time eliminates runtime initialization

**Rust backend mapping**: `comptime for` → manual unrolling or `seq_macro`; `comptime if` → `#[cfg(...)]`; `comptime` blocks → `const` evaluation.

**Design principle**: The Windjammer compiler expands these at compile time, so the developer writes clear intent and gets optimal code. This is "compiler does the hard work" in action.

**IMPORTANT: Automatic unrolling vs manual annotation**

Unlike Mojo's `@parameter for` which requires the developer to annotate loops for unrolling, Windjammer should **automatically** detect and unroll small constant-bound loops as a compiler optimization. This aligns with our core philosophy: "the compiler does the hard work, not the developer."

**Automatic unrolling criteria** (no annotation needed):
1. Loop has a compile-time-known iteration count (constant bounds)
2. Iteration count is small (e.g., ≤ 16 iterations)
3. Loop body is sufficiently simple (no complex control flow)
4. Loop body doesn't have side effects that depend on iteration order

```windjammer
// Developer writes natural code:
for i in 0..4 {
    process(data[i])
}

// Compiler AUTOMATICALLY unrolls to:
// process(data[0])
// process(data[1])
// process(data[2])
// process(data[3])
```

The explicit `comptime for` syntax is still available for cases where the developer wants to force unrolling on larger loops or when automatic detection wouldn't trigger. But the common case (small loops with constant bounds) should Just Work without annotation.

**Implementation approach**: Add an optimization pass in the Windjammer compiler that detects eligible loops before codegen and emits unrolled code. For the Rust backend, this means generating repeated statements instead of `for i in 0..N { }`. For Rust specifically, LLVM may already unroll these, but explicit unrolling gives us guaranteed behavior across all backends (Go, JS, Interpreter).

### 2. First-Class SIMD Type (Priority: HIGH)

**Mojo's approach**: `SIMD[DType.float32, 4]` maps directly to hardware vector registers. Operations automatically generate AVX/NEON instructions.

```python
# Mojo SIMD
let v = SIMD[DType.float32, 4](1.0, 2.0, 3.0, 4.0)
let result = v * 2.0  # Uses hardware SIMD multiply
```

**Windjammer adaptation**: A `simd` type that works seamlessly with our math types:

```windjammer
// SIMD vectors that map to hardware
let positions = simd<Vec3, 8>([...])
let velocities = simd<Vec3, 8>([...])

// Automatic SIMD operations (compiler generates AVX/NEON)
let new_positions = positions + velocities * dt

// Works with game-specific types
let colors = simd<Color, 4>([red, green, blue, white])
let blended = colors.lerp(target_colors, 0.5)
```

**Why this matters for Windjammer**:
- Particle systems process thousands of particles per frame
- Physics engines benefit from batch vector operations
- Voxel rendering requires bulk data transformation
- SVO traversal and ray marching can process multiple rays simultaneously

**Rust backend mapping**: `simd<T, N>` → `std::simd::Simd<T, N>` (nightly) or `packed_simd` / manual intrinsics. The compiler selects the best available backend.

**Design principle**: Developers express *what* they want (parallel math on 8 vectors). The compiler figures out *how* (AVX-512 on x86, NEON on ARM, scalar fallback otherwise).

### 3. Compile-Time Parameter Values (Priority: MEDIUM)

**Mojo's approach**: Functions can accept both types AND values as compile-time parameters:

```python
# Mojo parameterized function
fn repeat[count: Int](value: String) -> String:
    var result = ""
    @parameter
    for i in range(count):
        result += value
    return result

let greeting = repeat[3]("hello ")  # Unrolled at compile time
```

**Windjammer adaptation**: Extend generic syntax to support const generics:

```windjammer
// Const generic parameters
fn create_buffer<const N: int>() -> Buffer<N> {
    Buffer::new(N)
}

// Compile-time sized arrays
struct FixedVec<T, const N: int> {
    data: [T; N],
    len: int,
}

// Game-relevant: compile-time physics step count
fn integrate_physics<const SUBSTEPS: int>(world: World, dt: float) {
    let sub_dt = dt / SUBSTEPS as float
    comptime for _ in 0..SUBSTEPS {
        world.step(sub_dt)
    }
}
```

**Why this matters for Windjammer**:
- Fixed-size buffers without heap allocation (critical for game performance)
- Compile-time unrolled physics substeps
- Buffer sizes known at compile time enable stack allocation
- Shader uniform buffer layouts with compile-time size validation

**Rust backend mapping**: Direct mapping to Rust const generics (`fn foo<const N: usize>()`). This is well-supported in stable Rust.

### 4. ASAP Destruction Policy (Priority: MEDIUM)

**Mojo's approach**: Values are destroyed "as soon as possible" — immediately after their last use, even within a function body. This is more aggressive than Rust's scope-based drop.

```python
# Mojo ASAP destruction
fn process():
    let buffer = create_large_buffer()  # Allocated
    let result = transform(buffer^)     # buffer moved, destroyed after transform
    # buffer already freed here, even though we're still in the function
    do_other_work()  # More memory available
```

**Windjammer consideration**: We already infer ownership. ASAP destruction could be an optimization the compiler applies automatically:

```windjammer
fn process() {
    let buffer = create_large_buffer()
    let result = transform(buffer)
    // Compiler inserts drop(buffer) here if buffer is no longer used
    // No annotation needed — compiler detects last use
    do_other_work()
}
```

**Why this matters for Windjammer**:
- Game engines are memory-constrained (16.67ms frame budget)
- GPU buffers should be freed immediately after use
- Texture uploads should release staging buffers ASAP
- Reduces peak memory usage in asset loading pipelines

**Rust backend mapping**: Rust already supports explicit `drop()`. The compiler can insert `drop()` calls after last use, or use `{let buffer = ...; transform(buffer)}` scoping.

**Design principle**: The developer writes natural code. The compiler optimizes memory lifetime. This is pure "compiler does the hard work."

## Features NOT Recommended

### Autotuning

**Why not**: Mojo removed this from the language itself (now a library concern). The complexity of compile-time performance search doesn't justify the language-level support. Windjammer can achieve similar results through:
- Profile-guided optimization (PGO) at the Rust level
- Benchmark-driven manual tuning
- Future: a `wj bench` plugin that suggests optimizations

### MLIR Backend

**Why not**: Windjammer's multi-backend strategy (Rust, Go, JS, Interpreter) is fundamentally different from Mojo's MLIR approach. MLIR would be a massive architectural change with minimal benefit given our Rust interop strategy. The Rust backend already provides access to LLVM optimizations.

### Python Compatibility

**Why not**: Windjammer is not a Python superset. Our target audience is game developers who want Rust-level performance with less complexity, not Python developers who want more speed.

### `def` vs `fn` Duality

**Why not**: Mojo supports both dynamic (`def`) and static (`fn`) functions. Windjammer uses `fn` exclusively with type inference. Having two function declaration styles contradicts our consistency principle.

## Implementation Roadmap

### Phase 1: Const Generics (v0.48)
- Add `const N: int` parameter syntax to parser
- Map to Rust const generics in codegen
- TDD: Test with fixed-size arrays, buffer types

### Phase 2: Compile-Time Evaluation (v0.50)
- Add `comptime` block syntax
- Implement `comptime if` for conditional compilation
- Map to `#[cfg]` and `const` evaluation in Rust
- TDD: Test with look-up tables, feature flags

### Phase 3: Compile-Time Loop Unrolling (v0.52)
- Add `comptime for` syntax
- Implement loop unrolling in codegen
- TDD: Test with physics substeps, SIMD-like patterns

### Phase 4: SIMD Type (v0.54)
- Add `simd<T, N>` type to stdlib
- Integrate with Vec3, Vec4, Color types
- Map to `std::simd` or intrinsics in Rust
- TDD: Test with particle systems, bulk transforms

### Phase 5: ASAP Destruction (v0.56)
- Analyze last-use of variables
- Insert early drop points
- TDD: Test memory usage reduction in asset loading

## Comparison Matrix

| Feature | Rust | Mojo | Windjammer (Proposed) |
|---|---|---|---|
| Const generics | `const N: usize` | `count: Int` parameter | `const N: int` |
| Compile-time eval | `const fn` | `@parameter`, alias | `comptime {}` |
| Loop unrolling | Manual / macros | `@parameter for` | `comptime for` |
| SIMD | `std::simd` (nightly) | `SIMD[DType, N]` | `simd<T, N>` |
| Value destruction | Scope-based drop | ASAP destruction | ASAP (inferred) |
| Ownership | Explicit `&`, `&mut` | `read`, `mut`, `var`, `out` | Inferred (automatic) |
| Auto-derive | `#[derive(...)]` | `@value` | Automatic (no annotation) |

## Windjammer Philosophy Alignment

Each recommended feature aligns with our core principles:

1. **Compiler does the hard work**: `comptime` blocks, ASAP destruction, and SIMD auto-vectorization are all compiler-managed optimizations.

2. **80/20 rule**: Const generics and `comptime for` give 80% of C++ template metaprogramming power with 20% of the complexity.

3. **Explicit where it matters, inferred where it doesn't**: SIMD type is explicit (performance intent matters). Destruction timing is inferred (mechanical detail).

4. **Backend agnostic**: All features map cleanly to Rust, and conceptually to Go/JS (with runtime fallbacks where hardware SIMD isn't available).

5. **No workarounds**: Each feature solves a real performance problem that game developers face, rather than being a theoretical exercise.

## Open Questions

1. Should `comptime for` support runtime-determined bounds with a compile-time maximum? (e.g., `comptime for i in 0..min(n, 8)`)
2. Should SIMD types be part of the language or stdlib? (Mojo chose language-level; we lean toward stdlib with compiler awareness)
3. Should ASAP destruction be opt-in (`@eager_drop`) or always-on? (We lean toward always-on with escape hatch)
4. How should `comptime` interact with the WJSL shader pipeline? (Shader variant generation is a key use case)

## References

- [Mojo Manual](https://docs.modular.com/mojo/manual/)
- [Mojo SIMD Documentation](https://docs.modular.com/mojo/std/builtin/simd/SIMD/)
- [Mojo Parameterization](https://docs.modular.com/mojo/manual/parameters/)
- [Mojo Value Lifecycle](https://docs.modular.com/mojo/manual/lifecycle/)
- [Mojo Compile-Time Metaprogramming](https://deepengineering.substack.com/p/building-with-mojo-part-4-compile)
- [Mojo Lifetimes and Origins](https://docs.modular.com/stable/mojo/manual/values/lifetimes/)
