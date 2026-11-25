---
status: proposal
last_verified: 46e3e530143f
---
# WJSL modules and CPU twins: a composable shading library

> **Proposal: stages S2 and later are not implemented.** Stages S0 (`wj build` resolves text includes) and S1 (same-folder `use crate::module::item`, `pub`, dead-code elimination, stable `module__name` names, WJ0197 to WJ0201) are implemented; resolver invariants are in [wjsl-modules-soundness.md](wjsl-modules-soundness.md). CPU twins, packages, subfolders and the parity harness are designs only.

Related: [ADR-0040](../adr/0040-explicit-module-reexports.md) (explicit re-exports),
[ADR-0039](../adr/0039-stdlib-one-way.md) (one way, no back-compat), [ADR-0021](../adr/0021-effects-and-capabilities.md)
(effects), [ADR-0017](../adr/0017-semantic-ir-for-all-backends.md) (one semantic layer),
[ADR-0030](../adr/0030-interpreter-rejects-what-rustc-rejects.md) (interpreter equals compiled),
[ADR-0002](../adr/0002-local-mutability.md) (explicit `let mut`), [Module system](../MODULE_SYSTEM.md),
[Shaders chapter](../book/src/tutorial/shaders.md), [WJSL RFC](../proposals/rfcs/WJ-LANG-01-shader-language.md)
(its open question on `import` is the one this page answers), `src/wjsl/MODULE.md`, `src/codegen/wgsl/MODULE.md`.

## 0. What is true today

| Fact | Evidence |
|---|---|
| WJSL has a textual include: `use "path"` or `#include "path"` inlines a file before lexing, with cycle detection and dedup by canonical path | `src/wjsl/includes.rs` |
| `wj validate-wjsl` resolves includes; `wj build FILE.wjsl --target wgsl` does not (it calls `transpile_wjsl`, not `transpile_wjsl_with_includes`), so a file that uses another fails to build | `src/bin/wj_cli_cmds.rs:616`, `src/cli/build.rs:442` |
| The WJSL lexer has no `use` or `pub` keyword; there are no module paths, no visibility, no namespaces | `src/wjsl/lexer.rs` |
| Every function of an included file is emitted; two files that declare the same name cannot both be included | `src/wjsl/codegen.rs` emits all functions |
| A second route, ordinary `.wj` with `@vertex`/`@fragment`/`@compute`, compiles to WGSL through `WgslBackend` | `src/codegen/wgsl/` |
| The compiler does not depend on `naga` or `wgpu`; the engine host validates WGSL with `naga` | `Cargo.toml`; the engine's runtime host |
| The engine's shaders share helpers through `use "wjsl_std/..."` text includes, and BRDF helpers such as `fresnel_schlick` and `distribution_ggx` are copied between shader files (find them with `grep -h '^fn ' *.wjsl \| sort \| uniq -c` in the engine's shader folder) | engine shader folder |
| Downstream character work keeps hand-written CPU twins of its skin, hair and eye shading next to the `.wjsl` files, so the two copies can drift | downstream report |

The gap is therefore not "no include at all" but: no **module** (names, visibility, dead-code elimination, stable
output names), the build path ignores the include that exists, and nothing lets the same source run on the CPU.

## 1. Goals and non-goals

| Goals | Non-goals |
|---|---|
| G1. A `.wjsl` file imports `pub` functions, structs and consts from another `.wjsl` file by module path, with the same `use` spelling as Windjammer | A WGSL-level linker or multiple WGSL modules per pipeline (WGSL has one module per shader; we emit exactly one) |
| G2. One emitted WGSL module per entry file, containing only what the entry points reach, in a deterministic order with stable, readable names | Preprocessor macros, conditional text inclusion, `#ifdef` permutations (specialization is S5, typed) |
| G3. The same shading source compiles to ordinary Windjammer (a CPU twin) that `wj test`, bakers and the CPU path tracer call, interpreted or compiled | Running arbitrary GPU code on the CPU: derivatives, workgroup memory, atomics and barriers stay GPU-only |
| G4. A parity harness that runs one function on the twin and on a GPU (or a software adapter) and compares within declared tolerances | Bitwise float equality between CPU and GPU (impossible under WGSL's float latitude; integer paths are bitwise) |
| G5. A versioned shading-library package (BRDF, skin, hair, cloth, noise) that any renderer backend and the web build share | Shipping a material-graph editor; graphs only generate WJSL that imports the library |
| G6. The compiler stays light: no `wgpu` in `wj`; `naga` only where validation needs it | A second shader language for CPU twins; the twin is generated, never hand-written |

## 2. The module model

### 2.1 Recommendation: Windjammer's module rules, applied to `.wjsl` files

One module system, two file kinds. A `.wjsl` file is a module exactly as a `.wj` file is; the resolver keys are the same
`(package, module path, item)` triples. This is what keeps S1 from blocking S2: the CPU twin of `brdf.wjsl` is the
Windjammer module at the same path.

| Aspect | Rule | Why |
|---|---|---|
| File to module | `brdf.wjsl` is module `brdf`; `lib/brdf.wjsl` is `lib::brdf` (directory layout, as Windjammer auto-discovery) | no `mod` boilerplate in shaders, identical to `.wj` discovery |
| Root (`crate`) | the shader root: the entry file's folder (S1), later `[shaders] root` in `wj.toml` or the package root | location-agnostic: never the clone path or folder name |
| Import | `use crate::brdf::fresnel_schlick`, `use crate::brdf::{fresnel_schlick, distribution_ggx}`, `use crate::brdf` then `brdf::fresnel_schlick(...)`; `super::` / `self::` as in Windjammer; a dependency package by its name, `use pbr_shading::skin::eval_skin` | the spelling an agent already knows from `.wj` |
| Visibility | items are private unless `pub`; using a private item is the same visibility error Windjammer reports, with the same "declare it `pub`" fix | one rule across file kinds |
| Re-exports | only explicit `pub use` (ADR-0040); `pub use brdf::*` is an explicit glob | a library's `lib.wjsl` lists its API |
| Glob import | `use crate::brdf::*` allowed in shader files, as in `.wj` | convenience for prelude-like modules |
| What a non-entry module may declare | `pub`/private `fn`, `struct`, `const` (later `override`, S5) | library code is pure: it never owns bindings |
| What only an entry file may declare | `@group/@binding` bindings, `var<private>`/`var<workgroup>` globals, `@vertex`/`@fragment`/`@compute` entry points | bindings and pipeline layout belong to the renderer backend, and have no CPU meaning |
| Cycles | an import cycle between files is an error listing the chain (as `includes.rs` does today); recursion inside a module is already illegal in WGSL | WGSL forbids recursion; a cycle never buys anything |
| Name lookup order | local item, then explicit `use`, then glob `use`, then the shader prelude; ambiguity between two globs is an error naming both | as `.wj` |
| String include | `use "path"` / `#include "path"` stays only until the engine is migrated by `wj fix` (rewrites each include into `use crate::...::*`), then is deleted (ADR-0039) | one way |

Alternatives considered:

| Option | Verdict |
|---|---|
| A. Keep the text include, just wire it into `wj build` | do it now as S0 (a bug fix), but it is not a module system: no DCE, no namespaces, collisions are fatal, no twin story |
| B. A shader-specific `import "x.wjsl" as x` | rejected: a second spelling for agents to learn, and the twin would need path translation |
| C. Windjammer `use` paths over `.wjsl` files (above) | **recommended** |
| D. Write shared shading code only in `.wj` and compile it through `WgslBackend` | the long-term convergence point (section 3.1), not the first step: `.wj` has no vector types or swizzles today, and downstream shaders are `.wjsl` |

Example (same folder, the S1 slice):

```text
// shaders/brdf.wjsl
pub fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}
fn unused_helper() -> f32 { return 0.0; }

// shaders/skin.wjsl
use crate::brdf::fresnel_schlick
@fragment
fn fs_skin(@location(0) n: vec3<f32>, @location(1) v: vec3<f32>) -> @location(0) vec4<f32> {
    let f = fresnel_schlick(clamp(dot(n, v), 0.0, 1.0), 0.028);
    return vec4<f32>(vec3<f32>(f), 1.0);
}
```

`wj build shaders/skin.wjsl --target wgsl -o out` writes one `out/skin.wgsl` containing `brdf__fresnel_schlick` and
`fs_skin`, and no `unused_helper`.

### 2.2 Which items end up in the emitted WGSL

| Step | Rule |
|---|---|
| Entry selection | the file named on the command line is the entry file; each of its `@vertex`, `@fragment`, `@compute` functions is a root. `wj build DIR --target wgsl` builds every file that declares at least one entry point; library files are checked, not emitted |
| Reachability | from the roots plus every item declared in the entry file (unchanged behavior for today's single-file shaders), follow calls, type uses (struct fields, parameters, returns, constructors) and const uses across modules |
| Dead-code elimination | an imported item not reached is not emitted; unused items in the entry file are still emitted (no change to existing goldens) |
| Ordering | WGSL allows module-scope declarations in any order, so order is purely for diffs: imported modules in depth-first post-order of the import graph with imports sorted by module path, items in source order within a module, then the entry file in source order |
| Determinism | output depends only on source text and module paths; never on directory listing order, absolute paths, hash iteration or what else is in the folder |
| One printer | all printing goes through `wgsl_printer` and `src/wjsl/codegen.rs` (the invariant in `src/wjsl/MODULE.md`) |

### 2.3 Names: stable, readable, collision-free

WGSL has no namespaces. The emitted name of an item is a pure function of where it is declared:

| Item declared in | Emitted name | Example |
|---|---|---|
| the entry file | its own name (entry points must keep host-visible names) | `fs_skin`, `SkinUniforms` |
| module `m::n` of the same package | `m__n__name` | `brdf__fresnel_schlick`, `lib__noise__fbm3` |
| module `m` of dependency package `p` | `p__m__name` | `pbr_shading__skin__eval_skin` |

- **Stable:** never depends on collisions or on the set of files in the build, so adding a file never renames anything
  in existing output, and hosts, captures (RenderDoc, browser devtools) and goldens see the same names across builds.
- **Injective:** WJSL identifiers may not contain `__` (a new diagnostic; the engine's shaders use none today), and
  WGSL reserves only a leading `__`, so `a__b` cannot collide with a user name.
- **Readable:** the module path is visible in GPU tools; an optional comment line `// from brdf.wjsl:2` above each
  imported item (on by default, stripped by a future `--strip-comments` for size).
- **Package versions are not in names:** one version of a package per shader build (the resolver refuses two).
- **Shared with the twin:** the twin keeps the module path itself (`brdf::fresnel_schlick`), and the GPU name is
  derived from the same key, so parity reports print both from one source of truth.

### 2.4 Diagnostics S1 needs (codes allocated when implemented)

| Situation | Message shape |
|---|---|
| unknown module | "no module `brdf` in `crate` (looked for `brdf.wjsl` next to `skin.wjsl`)" with the real line and column |
| item not found / not `pub` | "`helper` is private to `brdf`"; fix: add `pub` |
| binding or entry point in a non-entry module | "bindings belong to the entry file; pass the resource as a parameter" |
| import cycle | the chain, as `includes.rs` prints it today |
| `__` in an identifier | "reserved for module-qualified names; rename" |
| two globs bring the same name | names both modules; fix: import it explicitly |

## 3. CPU twins

### 3.1 How the same source becomes Windjammer

Recommendation: **one checked WJSL module, two printers.** The WJSL front end (parser, checker, module graph) stays
the single owner of shader semantics; a new printer emits each library module as a generated `.wj` module against a
**shader prelude**. A `.wj` file then imports a `.wjsl` module directly (`use crate::shaders::brdf::fresnel_schlick`):
the Windjammer resolver sees `.wjsl` files as modules and asks the WJSL front end for their twin. No twin is ever
written or committed by hand; the generated `.wj` lives in the build cache.

| Alternative | Why not first |
|---|---|
| Teach `.wj` vector types, swizzles and WGSL builtins, and compile shared code through `WgslBackend` (option D) | the end state under ADR-0017 (one semantic IR, WGSL as a backend), but it needs vector types in the core language and a second WGSL path to reach WJSL's checks; revisit after S3 with parity data |
| Interpret WJSL directly in the Windjammer interpreter | a second evaluator for one language; compiled mode would still need a printer |
| Keep hand-written twins plus parity tests | the status quo; drift is the problem |

### 3.2 The shader prelude (identical semantics on both targets)

| WJSL | CPU twin (prelude) | Note |
|---|---|---|
| `f32`, `i32`, `u32`, `bool` | `f32`, `i32`, `u32`, `bool` with explicit widths | every literal and binding typed explicitly in the generated code: no reliance on numeric inference |
| `vec2/3/4<T>`, `mat2x2..mat4x4<f32>` | prelude structs `Vec2<T>`..., `Mat4` with `impl Add/Sub/Mul/Div`, scalar broadcast and component-wise ops | column-major, WGSL operator meaning (`m * v`, `v * m`) |
| swizzle read `v.xzy`, `c.rgb` | lowered by the printer to a constructor of fields (`Vec3::new(v.x, v.z, v.y)`) | no thousand generated methods |
| swizzle or component write `v.xy = e` | lowered to per-field assignments of a temporary | the WJSL checker already forbids repeated components in writes |
| `let` / `var` | `let` / `let mut` (ADR-0002) | |
| builtins: `abs clamp mix step smoothstep min max dot cross normalize length distance reflect refract pow exp exp2 log log2 sqrt inverseSqrt sin cos tan atan2 floor ceil fract round sign select saturate fma` | prelude functions with WGSL definitions (e.g. `clamp(e, lo, hi) = min(max(e, lo), hi)`, `mix(a, b, t) = a * (1 - t) + b * t`, `round` = ties to even, `fract(x) = x - floor(x)`) | one table: name, WGSL signature, CPU body, accuracy bound (feeds S3 tolerances) |
| integer `/` `%` by zero, overflow, shifts | WGSL results (division by zero yields the dividend, wrapping arithmetic, shift amount masked) | integer paths must be bitwise equal |
| `textureSampleLevel`, `textureLoad`, `textureDimensions` | a `Texture2D` / `Sampler` pair of prelude traits with a CPU implementation over image data (nearest, linear, clamp, repeat, mirror; explicit level) | S4 prerequisite, see below |
| `textureSample`, `dpdx`, `dpdy`, `fwidth`, `workgroupBarrier`, atomics, `var<workgroup>` | **no twin**: a function that uses them (directly or through callees) is GPU-only | inferred, like an effect: twin-eligibility is a fact per function (ADR-0021 style), never a name list |

Rules for shared code:

- A library function is twin-eligible when it and everything it calls avoid the GPU-only rows above. A `.wj` call to
  a GPU-only function is an error naming the call chain to the offending builtin.
- Textures cross into shared code as parameters (WGSL allows texture and sampler parameters) and use explicit-level
  sampling; implicit-derivative sampling stays in entry files, which pass the sampled value in.
- Twin functions have no effects; the effect pass must infer the empty set for every generated twin module.
- The prelude lives where the compiler versions it with the printer: recommended `std::shader` (open question 2).

### 3.3 Float behavior, stated rather than hoped for

| Behavior | GPU (WGSL) | CPU twin | Parity treatment |
|---|---|---|---|
| contraction of `a * b + c` into FMA | allowed to the implementation | never; `fma()` only when written | tolerance covers one rounding per contracted op |
| NaN and infinity | implementations may assume they do not occur | IEEE | parity domains are finite; NaN/inf behavior is tested CPU-only |
| denormals | may flush to zero | IEEE | comparator treats magnitudes below the smallest normal `f32` as zero |
| transcendental accuracy | per-builtin bounds in the WGSL spec (absolute for `sin`/`cos` on a range, ULP for others) | Rust `f32` library | per-builtin bound from the prelude table, composed along the function |
| `f16` (S5+) | when the adapter has it | emulated by rounding to `f16` after each op | separate tolerance class |
| integers | defined (wrapping) | same | bitwise equal |

The interpreter must evaluate `f32` with `f32` rounding (not `f64`) for the twin to mean anything; S2's gate checks
interpreted against compiled bitwise (ADR-0030) on the same inputs.

### 3.4 Parity harness (S3)

| Piece | Design |
|---|---|
| Declaring a parity test | a `.wj` test names the function, an input domain per parameter (seeded ranges, unit vectors, normalized pairs) and a tolerance (`abs`, `rel`, `ulp`, or `derived` from the prelude table) |
| CPU side | calls the twin, interpreted and compiled |
| GPU side | the compiler emits a wrapper `@compute` kernel per function under test (storage buffer of inputs, buffer of outputs), using the same module graph and names |
| Dispatch | through a headless compute port outside the compiler (the engine's or a small ecosystem runner): a hardware adapter where present, otherwise a software adapter (llvmpipe, WARP, SwiftShader) |
| No adapter | the GPU half is reported as **skipped**, never as passed; `naga` validation of the wrapper still runs |
| Report | worst error per output component, the input that produced it, both names (twin path and GPU name) |
| Gate on the gate | a planted fault (twin printer swaps `mix` arguments; a constant changes) must turn the harness red |

## 4. Renderers, web and the shading library package

| Concern | Design |
|---|---|
| Swappable renderers | the library is functions only (BRDF lobes, skin, hair, cloth, noise); each backend's entry files own bindings and layout and `use` the library. The wgpu backend, a future pure-Windjammer GPU backend and the CPU path tracer (through twins) evaluate the **same** lobe code |
| Render conformance | the CPU path tracer built on twins is the oracle every backend's conformance suite compares against; per-function parity (S3) localizes a failing image to a lobe |
| Web | WGSL is WebGPU's native language, so the browser build loads the same emitted module; no translation step differs between native and web |
| Package | an ecosystem package (working name `pbr_shading`) of `.wjsl` modules with a `lib.wjsl` of explicit `pub use`, semver-versioned through the normal package source of truth ([ADR-0010](../adr/0010-package-source-of-truth.md)); twins come with it, so a downstream tool that runs as an engine plugin or standalone uses one dependency in both modes |
| Material graphs vs hand-written functions | graphs never concatenate WGSL strings: a graph compiles to a generated entry `.wjsl` that imports library blocks and wires their parameters; the same graph evaluates on the CPU through the twins. Hand-written functions and graph nodes are the same library items |
| Host-shared structs | uniform and storage structs shared with host `.wj` code keep the existing layout checks; library structs used in bindings get a stable name per 2.3 |

## 5. Staged ladder

Each stage ships alone and is valuable alone. Sizes are agent-days of implementation and tests, excluding review.

| Stage | Scope | Size | Gate |
|---|---|---|---|
| S0 | `wj build FILE.wjsl` resolves the existing includes (call the include-resolving path, base dir = the file's folder) | 0.5 | a build of a file with `use "x.wjsl"` succeeds; a mutation reverting the call fails the test |
| **S1** | same-folder `use crate::m::item` between `.wjsl` files, `pub`, one WGSL module, DCE, deterministic order, stable `m__item` names, `naga` validation in tests | 3-4 | acceptance tests below |
| S1b | dependency packages and subfolders (`lib::noise`, `pkg::m`), `pub use` re-exports, `wj fix` migration of `use "path"` includes, then deletion of the text include | 2-3 | engine shaders migrated and byte-identical except for names; text include gone |
| S2 | CPU twins of the pure subset: prelude, twin printer, `.wj` imports of `.wjsl` modules, twin-eligibility inference | 6-8 | downstream skin/hair/eye helper functions called from `wj test`, interpreted equals compiled bitwise; hand-written twins deleted downstream |
| S3 | parity harness: wrapper kernels, headless runner, tolerance table, skip reporting | 4-6 (+ compute port) | every library function has a parity test; planted twin fault turns it red; CI records skip vs pass |
| S4 | shading library package: GGX/Smith/Schlick, Burley diffusion, dual-lobe skin, a melanin-based hair fiber model, sheen, noise; `Texture2D` trait with CPU sampler | 8-12 | white furnace per lobe, reciprocity, F0 bounds, parity per lobe, a probe scene CPU oracle vs GPU within a per-pixel bound |
| S5 | specialization: WGSL `override` pipeline constants and compile-time `const` parameters, mirrored on the twin as parameters | 3-5 | one library function specialized two ways, both validated, both at parity |

### 5.1 S1 decisions that must not block S2 (the twin)

| Decision made in S1 | Constraint so S2 can reuse it |
|---|---|
| Module resolver | keyed by `(package, module path, item)`, the same key the Windjammer resolver uses; built as a module graph that S2 can hand to the `.wj` resolver, not as text splicing |
| Mangling | a pure function of that key; the twin keeps the key's path, so both names come from one place |
| Visibility | the Windjammer `pub` rule and the same fix text, so a `.wj` importer gets identical errors |
| Library-module content rule | no bindings, globals or entry points in non-entry modules: exactly what a twin can represent |
| AST | per-module `ShaderModule` plus a resolved symbol table, not one merged AST; the twin printer prints per module |
| Checker | type-checks each module against the signatures it imports (no re-check of inlined text), so the twin sees fully typed items |

### 5.2 S1 acceptance tests (the first branch)

| Test | Asserts |
|---|---|
| same-folder use emits one module | fixture `brdf.wjsl` + `skin.wjsl`: one `skin.wgsl`; contains `brdf__fresnel_schlick` once and no `brdf__unused_helper`; golden in `tests/golden/wgsl/` |
| `naga` validates every new golden | `naga` as a dev-dependency of the test crate only, parse plus `Validator` with all flags |
| names are stable | adding a third file with a `fresnel_schlick` of its own leaves the output bytes unchanged |
| qualified use | `use crate::brdf` then `brdf::fresnel_schlick(...)` emits the same bytes as the direct import |
| collision | two modules each declaring `pow5`, both reached: both emitted under distinct names |
| visibility | importing a private item fails with the `pub` fix at the right line and column |
| missing module, cycle, `__` identifier, binding in a library module | each fails with the message in 2.4 |
| deterministic and location-agnostic | building the fixture from two different absolute directories gives identical bytes |
| no regression | every existing WJSL golden is byte-identical (entry files keep all their items) |
| `wj validate-wjsl` | validates a folder of library modules without entry points, and entry files that import them |
| gate on the gate | disabling DCE or mangling turns the golden test red |

First branch: `wjsl-modules-s1`, off the current integration head, touching `src/wjsl/` (lexer `use`/`pub`, parser,
a module-graph module, checker symbol tables, codegen naming/DCE/order), `src/cli/build.rs` and
`src/bin/wj_cli_cmds.rs`, plus the tests above. S0 lands first as its own small commit on the same branch.

### 5.3 Compiler work by component

| Component | S1 | S2 | S3 | S5 |
|---|---|---|---|---|
| WJSL lexer/parser | `use` paths, `pub`, qualified calls `m::f()` | | | `override`, `const` params |
| WJSL module resolver (new) | same-folder graph, cycles, visibility | packages, re-exports; shared with `.wj` resolver | | |
| WJSL checker | per-module symbol tables, imported signatures | twin-eligibility inference | | |
| WGSL emitter | mangling, DCE, ordering, origin comments | | wrapper kernels | override emission |
| Windjammer resolver | | `.wjsl` files as modules | | |
| std | | `std::shader` prelude | tolerance table | |
| Interpreter | | `f32` rounding fidelity (verify) | | |
| Outside the compiler | | | headless compute runner | |

### 5.4 Risks

| Risk | Mitigation |
|---|---|
| Float parity is noisy (contraction, flushing, builtin accuracy, driver variance): flaky tests or tolerances loose enough to hide bugs | derived per-builtin tolerances, finite seeded domains, a software adapter as the CI reference, a planted-fault check per harness |
| Two semantic owners drift: the twin `.wj` is checked by the Windjammer checker, which may infer widths, mutability or ownership differently | the printer emits fully annotated types and explicit `let mut`; a corpus compiled both ways; long-term one semantic IR (ADR-0017) |
| GPU availability and build weight: no adapter on CI or on light machines, heavy `wgpu`/`naga` builds | `naga` as a test-only dependency, the runner outside `wj`, skipped is reported as skipped |
| Name churn: changing the mangling later breaks hosts and goldens | fixed in S1 as a pure function of the declaring path; changed only by a recorded decision |
| Engine migration from text includes touches many shaders | `wj fix` does it mechanically in S1b; byte-compare outputs before and after |

## 6. Open questions for the owner

| # | Question | Recommendation |
|---|---|---|
| 1 | Import spelling: Windjammer `use crate::m::item` (one way) or a shader-specific form? | Windjammer's form; the twin and agents both benefit |
| 2 | Where does the shader prelude live: `std::shader` or an ecosystem package? | `std::shader`: the twin printer targets it, so it must version with the compiler |
| 3 | Should prelude vector types become the engine's math types (one `Vec3`), or stay separate `f32` shader types? | one family: the engine's math consolidates onto the prelude types after S2, so CPU code and twins share values without conversion |
| 4 | Emitted names: always module-qualified (stable) or only on collision (shorter)? | always qualified; stability beats brevity, and origin comments keep it readable |
| 5 | Is the long-term home of shared shading code `.wjsl` or `.wj` with vector types (one IR, WGSL as a backend)? | `.wjsl` now; decide after S3 with parity data, aiming at one semantic IR |
| 6 | GPU half of parity in CI: require a software adapter, or allow skip? | require a software adapter on the Linux CI lane; allow skip only on developer machines, reported as skip |
| 7 | May the compiler depend on `naga` at run time to validate every build? | not by default (build weight); test-only in S1, an opt-in feature later if users ask |
