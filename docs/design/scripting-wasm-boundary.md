---
status: proposal
last_verified: dae10c39a885
---
# A language-neutral scripting boundary for the game engine (WebAssembly components and WIT)

> **Proposal: not implemented.** Nothing on this page exists in code yet. It designs how the game-host engine can host gameplay logic, mods and tools written in many languages through WebAssembly components described in WIT, with the Windjammer interpreter as a first-class script host. External tooling claims carry their source and the date checked (2026-11-25); claims we could not check say "not verified".

Related: [ADR-0021](../adr/0021-effects-and-capabilities.md) (effects, capabilities, sandbox),
[ADR-0034](../adr/0034-book-playground.md) (the interpreter compiled to wasm, running in a browser Worker),
[ADR-0024](../adr/0024-hot-reload.md) (hot reload at safe points),
[ADR-0038](../adr/0038-interpreter-world-class.md) (interpreter parity ladder),
[ADR-0031](../adr/0031-interpreter-crate-host.md) (crates through a host; notes on `wasm32-wasip2`),
[ADR-0007](../adr/0007-target-tiers-conformance.md) (target tiers), [ADR-0012](../adr/0012-rust-interop-boundary.md)
(the foreign-code boundary), [multi-target-codegen.md](multi-target-codegen.md), [INTERPRETER.md](../INTERPRETER.md).

## 1. Summary

| Question | Answer |
|---|---|
| Can the engine host C#, Lua, Rust, TypeScript, Python, Java, Kotlin, Ruby and more? | **Yes, through one WIT-described boundary.** Every language that can produce a WebAssembly component (directly, through an adapter, or by shipping its interpreter as a component) can implement the same worlds. The engine sees one kind of guest. |
| What does the boundary look like? | Two worlds. **`logic`**: the existing pure-reducer contract (`new-state`, `step`, `snapshot`, `query`, `run-log`; JSON text across the boundary), unchanged. **`frame`**: one `step` call per fixed tick that takes a frame snapshot in and returns command buffers out. No per-entity calls across the boundary. |
| Which runtime? | Native: Wasmtime (component model, fuel, NaN canonicalization, deterministic relaxed SIMD, all documented). Web: the browser's own WebAssembly engine plus glue we generate from our WIT. |
| Where does Windjammer itself fit? | Compiled Windjammer is a Tier 1 guest. The interpreter implements the same worlds in-process (native) and as the playground wasm (web), with hot reload at the frame boundary. Same goldens, same bytes, three ways. |
| Does the coordinator's tiering hold? | Mostly, with three corrections (section 5): AssemblyScript and Go move down a step (component tooling not verified / not maintained), JavaScript splits in two (QuickJS is light, SpiderMonkey-based componentize-js is heavy), and Tier 3 splits into AOT-compiled (.NET NativeAOT-LLVM) and WasmGC (Kotlin, Java) families with different costs. |
| First step | S0: write the `logic` world in WIT and add a Wasmtime adapter beside the two existing logic-host adapters; replay every existing golden through it byte for byte. |

## 2. Goals and non-goals

| Goal | What it means here |
|---|---|
| Per-frame gameplay logic | Real-time systems (AI, abilities, rules, camera logic) step once per fixed tick inside a frame budget, for AAA 3D, 2D and 2.5D games. |
| Turn and event logic | The pure-reducer contract a first game already uses: `step(data, state, input) -> (state, events)`, total, versioned state, golden replays. |
| Mods | Untrusted third-party code, sandboxed by construction: no import means no authority. |
| Tools | Editor and authoring-studio automation (import, bake, batch edit) with explicit file grants. |
| Headless tests | Every guest runs under the same golden replays without a window, GPU or browser. |
| Web parity | The same component bytes run natively and in the browser with the same results. |
| Own every interface | The WIT packages are ours (`wj:*`). Runtime types (Wasmtime, JS glue) stay inside adapters; engine and game code never see them. |
| Determinism | Same input log, same output bytes on every host, for replay, rollback netcode and save verification. |

| Non-goal | Why |
|---|---|
| Rendering or shading in scripts | Scripts emit render intents; renderers stay swappable behind the engine's render port. GPU code stays WJSL. |
| Calling engine internals per entity | Per-call cost would dominate; the boundary is batched by design (section 3.4). |
| Threads inside a script | Not in WASI 0.2/0.3; shared-everything threads is unimplemented in Wasmtime (section 4). Parallelism lives in the engine's job system. |
| A promise of every language at launch | Languages enter through a conformance suite, one at a time, as packages. |

## 3. The boundary

### 3.1 Worlds

| World (package `wj:engine`) | Guest exports | Guest imports | Who uses it |
|---|---|---|---|
| `logic` | `new-state`, `step`, `snapshot`, `query`, `run-log`, `state-version` | none | turn/event rules; the existing contract, unchanged |
| `frame` | `init`, `step`, `snapshot`, `restore` | `log`, `assets` (name to handle, read-only) | real-time gameplay scripts |
| `mod` | the `frame` exports | `frame` imports, plus only the interfaces the mod's grant names | untrusted mods |
| `tool` | `run(args) -> result` | `log`, `assets`, `scene-edit`, `wasi:filesystem` (preopened, grant-scoped) | editor and studio automation |

A guest that imports something its world does not offer fails to instantiate. That link failure is the capability check
(section 6.3).

### 3.2 The `logic` world (the existing contract as WIT)

```wit
package wj:engine@0.1.0;

interface logic-module {
  record step-result { state: string, events: string }   // JSON text, as the contract defines
  name: func() -> string;
  state-version: func() -> u32;
  new-state: func(data: option<string>) -> result<string, string>;
  step: func(data: option<string>, state: string, input: string) -> result<step-result, string>;
  snapshot: func(data: option<string>, state: string) -> result<string, string>;
  query: func(data: option<string>, state: string, name: string, args: string) -> result<string, string>;
  run-log: func(data: option<string>, inputs: string) -> string;
}

world logic { export logic-module; }
```

| Contract rule | How the WIT keeps it |
|---|---|
| `step` is total; a rule violation is a `rejected` event | `result::err` is reserved for "the host could not run the module" (bad JSON, trap), exactly as the logic host's `Err` today |
| State is opaque JSON text, `v` first, `data_hash` second | `state: string`; the host checks `v` against `state-version` and hashes `data` as it does now |
| Data is passed in by the host, never read from files | `data: option<string>`; the world imports nothing, so a guest *cannot* read files |
| Queries never change state | `query` returns a value, not a state; the conformance suite asserts the state is unchanged |
| Host side `boot_logic` / `command` / `observe` / `query` | unchanged: they are the *host's* API to the page or shell. The router, slots, version and hash refusal stay in the host. Only the module side becomes a component. |

So yes: the pure-reducer contract is expressible as a WIT world with no change to any module's rules or JSON shapes. It
adds `query` as a module function, which the logic host also needs for native modules.

### 3.3 The `frame` world (real-time)

| Interface | Direction | Shape | Notes |
|---|---|---|---|
| `frame-in` | host to guest, argument of `step` | record: `tick: u64`, `input: input-snapshot`, `events: list<event>`, `views: list<column-view>` | the whole frame's inputs in one value |
| `input-snapshot` | in | keys and buttons as bitsets, axes as `list<f32>`, pointer, text input | sampled by the host's never-dropping input queue at the tick |
| `time` | in | `tick` only (integer ticks at a fixed rate) | no wall clock; matches the integer-tick rule logic modules already follow |
| `column-view` | in | `component: u32`, `entities: list<u32>`, `data: list<u8>` (packed, schema-versioned) | stage S0 to S3: copied; S4: see 3.4 |
| `frame-out` | guest to host, result of `step` | record: `commands: list<u8>` (packed command buffer), `events: list<event>` | one value per frame |
| entity commands | out | spawn, despawn, set-component (by handle), add-tag | applied by the host after the step, in order |
| render intents | out | draw-mesh(asset-handle, transform), sprite, camera, light, debug-line | renderer-neutral; any render port backend consumes them |
| audio cues | out | play/stop/param by asset handle | consumed by the audio port |
| `assets` | guest import | `resolve(name) -> option<handle>`, `status(handle)` | read-only; loading is the host's job |
| save | none | the host stores `snapshot()` bytes through its save port | scripts never touch storage |
| `log` | guest import | `log(level, text)` | rate-limited by the host |

Handles are plain `u32`/`u64` values minted by the host, not WIT `resource`s, so a frame snapshot stays plain data
that can be logged, replayed and diffed. WIT resources are kept for the `tool` world, where lifetimes matter more than
replay.

### 3.4 Data model and the per-frame call budget

| Option | How | Cost | Determinism | Verdict |
|---|---|---|---|---|
| A. Canonical-ABI copies of packed columns | `list<u8>`/`list<f32>` arguments; the canonical ABI copies through linear memory | O(bytes) per frame, plus a fixed cost per call | exact | **default**, stages S0 to S3 |
| B. Guest-owned arena, host-mapped view | the guest exports a buffer; the host writes columns into the guest's memory before `step` and reads commands after | no copy into the guest beyond the host's write | exact | S4, behind a measurement; needs a core-module escape hatch (components are shared-nothing) so it is a Tier 1 privilege |
| C. Per-entity import calls | `get-position(entity)` per entity | a boundary crossing per entity per frame | exact | **rejected**; the cost scales with entity count |
| D. Shared memory between host and guests | shared-everything threads | n/a | n/a | not available: unimplemented in Wasmtime (section 4) |

Budget rule (a design target, to be ratcheted by measurement): **a script makes one `step` call per tick and a small,
constant number of imports**, independent of entity count. The per-frame script budget is a fraction of the frame
time that the engine scorecard owns; S0 measures the fixed cost per call and the cost per byte so the budget is set
from numbers, not guesses. Published figures for call overhead vary widely and some are disputed (a Wasmtime
community thread reports far higher import-call costs than a Wasmtime PR suggests; the component-model roadmap
reports extra overhead on synchronous calls from async bookkeeping); we measure our own.

## 4. Hosts

| Host | Choice | Verified facts (source, checked 2026-11-25) | Our use |
|---|---|---|---|
| Native runtime | **Wasmtime** | component model tier 1, enabled by default; GC, exception handling, memory64, relaxed SIMD, tail calls tier 1; threads tier 2 with known shared-memory issues; shared-everything threads unimplemented; stack switching tier 3 ([Wasmtime proposal status](https://docs.wasmtime.dev/stability-wasm-proposals.html)). WASI 0.3 released 2026-06-11 ([wasi.dev](https://wasi.dev/releases/wasi-p3)); Wasmtime enables 0.3 and component async by default from release 46 (secondary source, not verified against release notes) | one `Engine` per process, one `Store` per script instance, pooling allocator for mods |
| Metering | **fuel** | fuel is deterministic, epoch interruption is not ([Wasmtime: interrupting wasm](https://docs.wasmtime.dev/examples-interrupting-wasm.html)) | fuel per `step` for replay and netcode builds; epoch for editor tools where determinism is not needed |
| Deterministic mode | config knobs | `cranelift_nan_canonicalization`, `relaxed_simd_deterministic` (or disable relaxed SIMD), fixed-size memories or a store limiter to remove `memory.grow` nondeterminism, disable copy-on-write init for identical fuel across hosts, virtualize WASI clocks ([Wasmtime: deterministic execution](https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html)) | all on in replay/netcode builds; section 6 |
| Memory limits | store limiter | memories and tables bounded per store (same page) | per-world defaults; mods get the smallest |
| Platforms without JIT | Wasmtime's portable interpreter (Pulley) is documented ("Portable Interpretation", same docs) | performance not verified | consoles and locked-down mobile: measure before promising |
| Wasmer, WAMR | alternatives | Wasmer component-model support: not verified. WAMR: named as a working host by componentize-dotnet's README; component support not verified | not chosen; the adapter boundary keeps the choice reversible |
| Web | browser WebAssembly + **our generated JS glue** | no major browser runs components natively; `jco transpile` turns a component into core modules plus JS glue, and jco calls its browser support experimental ([jco](https://github.com/bytecodealliance/jco)). WasmGC on by default in Chrome 119, Firefox 120, Safari 18.2 ([Chrome blog](https://developer.chrome.com/blog/wasmgc)) | we generate small glue for *our* worlds (string and byte lists only), and use `jco transpile` as an oracle in tests |
| Web metering | no fuel in browsers | n/a | Tier 1 guests: a fuel-counting pass at load time (our tool, to build); interpreters: their own step counters (the playground already has fuel, ADR-0034); a Worker with a wall-clock kill for tools |

### 4.1 The interpreter as a first-class script host

| Aspect | Design |
|---|---|
| Native | An in-process adapter implements the `logic` and `frame` worlds on the interpreter directly (no component, no process per call: today's interpreted logic adapter spawns a process per call and is a parity tool only). |
| Web | The playground wasm ([crates/windjammer-playground](../../crates/windjammer-playground/)) already runs a program from source with no imports and its own fuel and output limits (ADR-0034). It gains exported `logic`/`frame` entry points so the same `.wj` source runs as a script in the page. |
| Hot reload | The frame boundary is the natural safe point of ADR-0024: between two `step` calls nothing is on the stack. Body-only edits swap in place; a changed type restarts the script from `snapshot()` text. Compiled guests in any language get the same behaviour by reinstantiating and calling `restore(snapshot)`. |
| Same semantics as compiled | The gate is the golden replay: interpreted, compiled-to-component and native static runs produce identical bytes (ADR-0038 parity ladder; the logic contract's own rule). |
| Why it matters | Edit a gameplay script and see it change in the running game, natively and in the browser, with no build step; ship the compiled component when the script settles. |

## 5. Per-language feasibility

Overhead class: **A** = native-compiled guest, a call is a boundary crossing plus a canonical-ABI copy; **B** = an
interpreter inside wasm, add dispatch per script operation; **C** = a large runtime inside wasm (start-up and memory
dominate). Size classes are expectations to measure in S3/S5, not measurements.

| Language | Route to a component | Runtime in the module | Overhead | GC story | Tooling and maturity (source) | Tier |
|---|---|---|---|---|---|---|
| Windjammer (compiled) | Rust codegen, then `wasm32-wasip2` with `wit-bindgen` | none beyond std | A | ownership, no GC | our toolchain; the runtime crate does not yet build for wasm32 from a generated crate (a known gap of the logic host) | **1** |
| Windjammer (interpreted) | in-process (native); playground wasm (web) | the interpreter | B | interpreter heap | ADR-0034 measures the playground bundle; size levers listed there | **1 (host)** |
| Rust | `wasm32-wasip2` + `wit-bindgen` | none | A | none | reference toolchain ([component model book](https://component-model.bytecodealliance.org/language-support.html)); Rust 2026 goals take `wasm32-wasip3` to tier 2 ([Rust goals](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html)) | **1** |
| C / C++ | `wit-bindgen c` + wasi-sdk, `wasm-tools component new` | libc | A | manual | documented in the component model book (same source) | **1** |
| Zig | core wasm, then `wasm-tools component new` with an adapter | none | A | manual | not listed in the component model book; route plausible, not verified | **1 (not verified)** |
| AssemblyScript | core wasm only | small | A | linear-memory GC (its own) | no component-model or WIT tooling found (not verified either way) | **2**: needs our own core-module adapter; a one-day spike decides |
| Go (TinyGo) | `-target wasip2` | TinyGo runtime | A | linear-memory GC | native wasip2 support from TinyGo 0.34 per the book; the book's TinyGo page says its component tools are not maintained and recommends the Go tooling ([TinyGo page](https://component-model.bytecodealliance.org/language-support/building-a-simple-component/tinygo.html)) | **2** |
| Go (official) | `wasip1` + `go:wasmexport` (Go 1.24), adapted to a component | Go runtime | A to C | linear-memory GC; goroutines stop when an export returns ([Go blog](https://go.dev/blog/wasmexport)) | no official `wasip2` port found (not verified as of Go 1.26) | **3** |
| Lua | Lua interpreter built for WASI, wrapped as one shared component implementing our worlds; scripts are data | small interpreter | B | Lua GC in linear memory | community WASI builds exist ([lua-wasi](https://github.com/nalgeon/lua-wasi)); component packaging is ours to build; sizes not verified | **2** |
| JavaScript / TypeScript (light) | Javy (embedded JS engine; QuickJS per project history, engine not named on its README) | small | B | engine GC in linear memory | Javy README: static modules under a megabyte, kilobyte-range per-script modules with dynamic linking ([Javy](https://github.com/bytecodealliance/javy)); component output not stated (not verified). TypeScript: strip types at build time (the playground already does, ADR-0034) | **2** |
| JavaScript / TypeScript (full) | `jco componentize` (componentize-js on StarlingMonkey, pre-initialized with Wizer) | SpiderMonkey | C | engine GC | Bytecode Alliance tooling; size figures in secondary sources conflict (not verified) ([componentize-js](https://github.com/bytecodealliance/componentizejs)) | **3** |
| Python (light) | MicroPython built for wasm | small | B | MicroPython GC | not verified | **2 (not verified)** |
| Python (full) | `componentize-py` (bundles CPython) | CPython | C | refcount + GC in linear memory | Bytecode Alliance project; imports resolved at build time only; output size not stated (not verified) ([componentize-py](https://github.com/bytecodealliance/componentize-py)) | **3** |
| Ruby (light) | mruby for wasm | small | B | mruby GC | not verified | **2 (not verified)** |
| Ruby (full) | ruby.wasm (CRuby on WASI preview 1), adapted to a component | CRuby | C | CRuby GC | WASI builds with JS interop are published; component output only explored in a pull request (not verified as shipped) ([ruby.wasm](https://github.com/ruby/ruby.wasm)) | **3** |
| C# / .NET | `componentize-dotnet` on NativeAOT-LLVM, WASI 0.2 component, .NET 10 | AOT-compiled runtime | A to C | .NET GC in linear memory | README warns the underlying pieces are under heavy development; build hosts listed are Linux and Windows ([componentize-dotnet](https://github.com/bytecodealliance/componentize-dotnet)) | **3** (AOT, so per-call cost may be class A: measure) |
| Kotlin | Kotlin/Wasm (`wasmWasi` target), WasmGC | Kotlin stdlib | A to C | **WasmGC** | component model support is early: a wit-bindgen fork around Kotlin 2.4.0 betas, per JetBrains community channels (not verified against release notes) ([Kotlin/Wasm](https://kotlinlang.org/docs/wasm-overview.html)) | **3** |
| Java | TeaVM WasmGC backend (primary target since 0.11), J2CL Wasm (experimental) | Java runtime subset | A to C | **WasmGC** | TeaVM targets browsers with a JS loader; WASI/component output not verified ([TeaVM](https://teavm.org/docs/wasm-gc-backend.html)) | **3 (web first)** |
| Swift, Dart, MoonBit | various | varies | A | varies | Dart and MoonBit appear in the component model book's language list; Swift not verified | later |

**WasmGC note.** WasmGC is on in all three browser engines and tier 1 in Wasmtime, which makes Kotlin and Java far
lighter than shipping a JVM. The canonical ABI still passes values through linear memory, so a WasmGC guest copies at
the boundary like everyone else; a GC-reference ABI option was not verified.

**Verdict on the hypothesis.** Tier 1 holds for Windjammer, Rust and C/C++ (Zig likely). AssemblyScript and TinyGo
move to Tier 2 until their component routes are proven. Tier 2 holds for interpreters shipped as wasm, with the key
design choice that **one interpreter component per language implements our worlds and takes scripts as data**, so a
game ships each interpreter once. Tier 3 holds as optional packages, split into AOT (.NET) and WasmGC (Kotlin, Java)
families whose per-call cost may turn out much better than "heavy" suggests: measure before ranking.

## 6. Determinism and security

### 6.1 Floats and the numeric contract

| Source of divergence | Native (Wasmtime) | Web | Rule |
|---|---|---|---|
| NaN bit patterns | `cranelift_nan_canonicalization` | engine-specific; cannot be forced | guests must not observe NaN payloads; the replay hash canonicalizes NaNs before hashing |
| Core float ops (`f32.add`, `mul`, `sqrt`, ...) | IEEE-exact by the Wasm spec | same | deterministic; no implicit fused multiply-add exists in core Wasm |
| Relaxed SIMD (`relaxed_madd`, ...) | `relaxed_simd_deterministic` or disabled | varies by CPU | rejected at load in replay builds (validator check) |
| Transcendentals (`sin`, `exp`, ...) | each guest's own libm | same | only functions from **one shared deterministic math package** count as bit-stable across languages; this ties into the planned float contract (no implicit FMA, identical bits interpreted and compiled) |
| `memory.grow` success | fixed-size memories or a store limiter | browser may fail growth under pressure | replay builds declare min = max memory |
| Threads | not available | not available | none in scripts |
| Clock, randomness | not imported | not imported | integer ticks in `frame-in`; seeded generator state lives in the script's own state, as logic modules already do |

### 6.2 Resource limits

| Limit | Mechanism | Default (design target, tuned by S4 measurements) |
|---|---|---|
| CPU per step | fuel (native), injected counter or interpreter fuel (web) | a per-world budget; exceeding it traps the step |
| Memory | store limiter, fixed max | per world; mods smallest |
| Output | command buffer and log size caps | per frame |
| Fault | a trap disables that script instance; the frame continues with the last good state and an engine event | never a crash of the game |

### 6.3 Capabilities (ADR-0021 mapped to WIT imports)

| ADR-0021 effect | Gameplay script (`frame`) | Mod (`mod`) | Tool (`tool`) |
|---|---|---|---|
| `fs.read`, `fs.write` | not imported | not imported; persistent data only through snapshots | `wasi:filesystem` with preopened directories from the grant |
| `net.connect`, `net.listen` | not imported | `wasi:http` outgoing only, host allowlist from the grant (opt-in, per mod) | per grant |
| `env.read`, `process` | not imported | not imported | `process` never; `env.read` per grant |
| `clock`, `random` | virtual: tick and seeded state | same | real clock allowed (tools are not replayed) |
| `console` | `log` import, rate-limited | same | same |
| `ffi` | impossible: a component can only call what it imports | same | same |

Grants use ADR-0021's manifest grammar (a mod's manifest lists `[capabilities]`); the host builds the linker with
exactly the granted interfaces, so an ungranted import is a link error at load, reported like ADR-0021's
`capability denied`. This is ADR-0021's "later the WASM target" sandbox, made concrete.

| Untrusted-mod threat | Mitigation |
|---|---|
| infinite loop, memory bomb | fuel, store limits, trap isolation |
| reading other mods' state | one store per mod; components are shared-nothing |
| host-API abuse (spamming spawns, log floods) | caps on command-buffer size and log rate per frame |
| side channels | out of scope for v1; noted as an open question |
| supply chain | mods are content-addressed; the save records each mod's hash next to `data_hash` |

## 7. How this fits what exists

| Existing piece | Relation |
|---|---|
| The engine's logic host (module trait with static and interpreted adapters) | **Unifies.** A third adapter, `WasmLogic`, implements the same trait over a `logic` component. The conformance suites and golden replays run unchanged against it. |
| The first game's logic contract (`new_state/step/snapshot/query`, JSON) | **Is** the `logic` world (3.2). No module changes; the game gains every Tier 1/2 language for future modules. |
| Engine ports (input, time, events, save, assets, audio, render) | The `frame` world is a data projection of them: input and time flow in, events, entity commands, render intents and audio cues flow out. The ports stay the engine's own traits; the WIT is derived from them and a gate checks they agree. |
| Swappable renderers | Render intents are renderer-neutral; the engine's wgpu renderer, the studio's renderer and a browser renderer all consume the same command buffer. Scripts never see GPU types. |
| The authoring studio as an engine plugin | Its modeling-command port becomes the `tool` world's `scene-edit` interface, so studio automation is scriptable in any language, and the studio still runs on its own. |
| Interpreter and playground (ADR-0034, ADR-0024) | First-class script host (4.1). |
| Effects and capabilities (ADR-0021) | The grant model for mods and tools (6.3). |

## 8. Staged ladder

Sizes are estimates in agent-days. Each stage is valuable alone and has a gate that must fail on a planted defect.

| Stage | Result | Gate | Size | Measure first |
|---|---|---|---|---|
| **S0** native `logic` world | `wj:engine` WIT package with the `logic` world; `WasmLogic` adapter on Wasmtime inside its own adapter crate; guests: the logic host's reference fake module and toy module rebuilt as Rust components. The first game's modules join when their compiled build and the wasm32 runtime gap clear | every logic-host suite and every golden replays byte-equal through `WasmLogic`; planted faults (wrong `v`, nonpure `query`, trap in `step`) fail; no Wasmtime type outside the adapter (type gate) | 4 to 6 | fixed cost of one `step` call and cost per byte at several payload sizes; instantiate time; component size |
| **S1** web `logic` world | generated JS glue for our worlds; the same component bytes in headless Chrome, Firefox and Safari; `jco transpile` as an oracle | the web golden replay passes on all three; glue generator output diffed against the WIT (a planted WIT change fails) | 4 to 7 | page cold start, bytes (brotli), per-call cost vs native |
| **S2** more languages, one suite | compiled Windjammer (when the runtime builds for wasm32), C, Zig; a one-day AssemblyScript spike; the interpreter in-process adapter and the playground wasm implementing `logic` | the conformance suite (toy and fake goldens) passes in every language; a language is listed only while green (shrink-only skip file) | 5 to 8 | size and call cost per language on the same module |
| **S3** interpreters as packages | Lua and QuickJS (Javy) components implementing `logic` with scripts as data; hot reload via `restore(snapshot)` | same suite, scripts ported per language; reload keeps state across an edit; fuel trap on a planted infinite loop | 6 to 10 | cold start of Lua-in-wasm vs native Lua; interpreter component sizes; per-step cost |
| **S4** `frame` world and ECS views | `frame` world, command buffers, column views by copy; then the guest-owned arena (option B) only if the copy misses the budget | a frame-replay golden (input log to command-buffer hash) byte-equal native, web, interpreted; a planted per-entity call pattern trips the crossing-count gate | 10 to 20 (real ECS needs the engine's native build green; can start on the port fakes) | copy cost at increasing entity counts; crossings per frame |
| **S5** heavy runtimes as optional packages | componentize-py, componentize-dotnet, Kotlin/Wasm, ruby.wasm, each a package with its own conformance run and size report; owner decides which ship | conformance suite green per package; size and cold-start rows recorded in the release report | 3 to 5 per language | component size, cold start, memory, per-call cost |

**Recommended first branch: `wasm-logic-host` in the engine repository (S0).**

| Acceptance test | Fails when |
|---|---|
| WIT parses and the generated host bindings compile | the WIT is malformed |
| every existing logic-host conformance case passes through `WasmLogic` with the reference fake component | the adapter maps a call wrongly |
| every golden of the toy module replays byte-equal through `WasmLogic` | any byte differs |
| planted faults: a component that changes state in `query`, one with the wrong `state-version`, one that traps in `step` | the suite misses any of them |
| deterministic config on: fuel enabled, NaN canonicalization on, relaxed SIMD off, fixed memory; two runs give identical fuel totals | fuel differs between runs |
| type gate: no `wasmtime::` path outside the adapter crate | a runtime type leaks |
| a measurement report: per-call fixed cost and per-byte cost, instantiate time, component bytes | the report is missing (no number is hand-typed into docs) |

## 9. Risks

| Risk | Signal | Mitigation |
|---|---|---|
| Boundary cost too high for per-frame scripts | S0/S4 measurements over budget | batching is the design; option B for Tier 1; the interpreter path for iteration |
| Browser glue and component tooling churn (WASI 0.2 to 0.3, jco browser support experimental) | glue breaks on a toolchain bump | we generate glue for our own narrow worlds (strings and byte lists), pin toolchains, keep `jco` as an oracle only |
| Determinism gaps on the web (NaN payloads, no fuel) | web and native replay hashes differ | NaN-canonical hashing, validator rejects relaxed SIMD, injected fuel; the frame-replay golden runs on both |
| Language tooling maturity (Tier 2 and 3 claims marked not verified) | a language fails the suite | languages enter only through the suite; skip file is shrink-only |
| Consoles and iOS forbid JIT | no Wasmtime JIT there | Pulley or AOT; measure before promising |
| Compiled Windjammer cannot target wasm yet | the runtime's wasm32 gap persists | S0 uses Rust guests; Windjammer joins in S2; the interpreter covers Windjammer meanwhile |

## 10. Open questions for the owner

| # | Question | Recommendation |
|---|---|---|
| 1 | Approve WebAssembly components with our own WIT worlds as the engine's single scripting boundary? | **Yes.** One boundary, every language, sandboxed by construction, web and native. |
| 2 | Native runtime: Wasmtime? | **Yes.** Its determinism and metering knobs are documented; keep it behind an adapter so it stays swappable. |
| 3 | Web: generate our own glue or depend on `jco transpile`? | **Generate our own** for our worlds (owning the interface); use `jco` as a test oracle. |
| 4 | Which languages are first-class (shipped and supported by us)? | **Windjammer, Rust, C/C++, Lua, TypeScript (via QuickJS)**; everything else as community packages that pass the suite. |
| 5 | Are mods a launch goal? | **Yes, as a world (`mod`) from S4**, with ADR-0021 grants; network off by default. |
| 6 | Per-frame scripting in the first game? | **No change now**: its logic stays turn/event-based on the `logic` world; `frame` comes with real-time systems. |
| 7 | Heavy runtimes (Python, .NET, Kotlin, Ruby): invest? | **Optional packages after S4**, ordered by measured size and call cost, not by popularity alone. |
