---
status: accepted
last_verified: 2059d7b1
---
# ADR-0021: Effects and capabilities

- **Status:** Accepted (2026-10-12, project owner: accepted for design; implementation is staged, see "Staged plan")
- **Date:** 2026-10-12
- **Related:** [ADR-0001](0001-ownership-truth.md), [ADR-0004](0004-concurrency-surface.md), [ADR-0009](0009-security-enforcement.md) (enforcement level; this ADR supplies the mechanism it chooses a level for), [ADR-0010](0010-package-source-of-truth.md), [ADR-0012](0012-rust-interop-boundary.md), ADR-0017 (one semantic layer, `wt/convergence`), ADR-0019 (processes, durable processes; `wt/expressive`), [WJ-SEC-01..04](../proposals/rfcs/SECURITY-FRAMEWORK-OVERVIEW.md), spec chapter [effects.md](../spec/effects.md), `src/ir/effects.rs`, `src/ir/safety_type.rs`, `src/interpreter/builtins/std_http.rs`, `src/interpreter/sched.rs`

## Summary
One mechanism, six uses. The compiler infers, for every function, the set of **effects** it may perform (filesystem,
network, environment, processes, clock, randomness, foreign code, user-defined effects). Effects are **facts** of
the single semantic layer (ADR-0001/ADR-0017), with provenance, never a second decision-maker. **Capabilities** are
the authority to perform an effect: granted at the root by the manifest (`wj.toml [capabilities]`) or the CLI, per
package, and passable/attenuable as ordinary values. From those two notions:

| Use | What it is in this design |
|---|---|
| (a) sandboxed execution of untrusted or agent-written code | the interpreter run with an empty root grant plus explicit grants; every sink call is checked |
| (b) fine-grained permissions | grants are refined by resource: hosts (and ports), path scopes, env names, program names, extern libraries |
| (c) compiler-checked determinism for `durable fn` | every effect inside a durable body is journaled at a boundary the compiler finds; the remaining non-determinism (shared state, unordered iteration) is a compile error |
| (d) record/replay and deterministic simulation testing | all effects pass through one port; record, replay and simulate are three implementations of that port |
| (e) capability-scoped tool binding for agents | `wj mcp bind` turns tool schemas into typed modules whose functions perform per-tool effects; agents write Windjammer against them and run in a sandbox whose grant lists those tools |
| (f) application policy | user-defined effects plus `[policy]` rules (`only_via`, `forbid`) checked over the provenance facts |

## Context (what is true today, with evidence)

- **Inference exists, coarse, name-heuristic.** `src/ir/effects.rs` classifies std calls by module name and
  signature shape (`fs` with unit return = `FsWrite`, `http` with a callback parameter = `NetIngress`, ...) and
  propagates them up the call graph (`EffectSolver`). The vocabulary (`src/ir/safety_type.rs` `Effect`) has
  `FsRead, FsWrite, NetEgress, NetIngress, ProcessSpawn, EnvRead, EnvWrite, Ffi, Custom`: **no clock, no randomness**,
  no resource refinement (host, path), no provenance.
- **Build enforcement exists only behind a manifest.** `src/output_generation.rs` fails the build with
  "capability violation(s)" only when `wj.toml` has `[app_capabilities] allow/deny` (`src/config.rs`), application-wide,
  printed with `eprintln!` (not the shared diagnostics schema; code `WJ0024` is catalogued with
  `front_end_checked: false`). `wj check` does not report effects. Calls are resolved by name (DefId resolution is
  convergence C3), so inference is unsound across same-named functions.
- **Interpreter: one capability port, the rest ambient.** `std::http` goes through a port (`HttpMode::{Deny, Mock, Real}`,
  `std_http.rs`): the network is refused unless `--allow-network` or `--mock-http FILE`. `std::fs`, `std::env`,
  `std::process` call the OS directly (`builtins/std_sys.rs`), with ambient authority.
- **Interpreter: determinism machinery exists.** `sched.rs` is a deterministic cooperative scheduler (seed 0 = FIFO,
  seed N = seeded interleavings at synchronization points, virtual clock for `sleep`); `--deterministic-time` removes the
  real clock; `std::random` is seeded with a fixed seed under deterministic time (`std_sys.rs` `rng()`).
  This is the substrate for replay and simulation; it is not yet connected to an effect log.
- **Security RFCs WJ-SEC-01..04 are drafts** (about 15k lines together) mixing a sound core (inferred effects,
  per-dependency allowlists, lock file) with a large product surface (prompts, learning preferences, demo modes,
  container generation). ADR-0009 (Proposed) recommends per-application enforcement now, per-dependency later, and
  an explicit `unknown` effect for externs.

The owner's direction (2026-10-12): effects and capabilities are the keystone feature; design now, implement after
convergence C2 (provenance) lands, inside the semantic layer.

## Decision

### 1. Effects are facts in the semantic layer
The semantic layer gains one fact family (added to the ADR-0017 table):

| Fact | Meaning | Producer |
|---|---|---|
| `effects(fn)` | the inferred effect set of a function, each element with provenance (rule ID, the call site that introduced it, `direct` or `via callee`) | effect pass (semantic layer) |
| `latent(fn, param)` | the function calls its function-typed parameter `param` (its effects are the argument's, see 4) | effect pass |
| `sink(site)` | the call at `site` is a sink: which effect, which resource argument | effect pass, from the sink table |
| `grant_check(site)` | `Proven` (statically inside the grant: emit nothing), `Runtime(scope)` (emit a guard), `Denied` (a diagnostic) | effect pass + manifest |
| `journal(site)` | inside a `durable fn`: this call is a journaled step boundary | durability pass |

Backends and the interpreter read these facts. No backend, and not the interpreter, classifies effects on its own:
the **sink table** (which std function performs which effect on which argument) is one table in the semantic layer
(replacing the name/shape heuristics in `src/ir/effects.rs`), and the interpreter's builtins look their sink kind up in
it. A test asserts that every interpreter builtin that reaches the OS has a sink-table entry (independent oracle:
the builtin registry versus the table).

The pass depends on resolved calls (convergence C3, DefId) for soundness and on provenance (C2) for explanations and
policies. It runs after ownership inference and reads `param_mode` (for the durable rule on mutated step arguments).
It never writes ownership facts.

### 2. Effect vocabulary (v1)
Effects are dotted names with an optional resource refinement. The vocabulary is closed for built-ins and open for
user-defined effects.

| Effect | Performed by | Refinement (resource) | Granted by default |
|---|---|---|---|
| `fs.read` | read a file, list a directory, metadata, `exists` | path scope (a directory or a file; globs `**`) | interpreter local profile: project directory |
| `fs.write` | write, append, create, remove, rename, copy target | path scope | interpreter local profile: project directory |
| `net.connect` | http client, TCP connect | host, optional port (`api.example.com`, `*.example.com`, `localhost:8080`) | no |
| `net.listen` | servers, bind | address (`127.0.0.1:8080`) | no |
| `env.read` / `env.write` | environment variables | variable name | no (local profile: no) |
| `process` | spawn a program, `process::exit` is **not** an effect | program name | no |
| `time` | read the wall or monotonic clock, `sleep`, timers | none | yes |
| `random` | entropy (`std::random`, `uuid::v4`) | none | yes |
| `console` | stdout/stderr output, stdin input | none | yes |
| `ffi` | calling an `extern` function whose effects are not declared (`unknown`) or declared `ffi` | library or crate name | no |
| user-defined | `@effect("ledger.post")` on a function | none in v1 | per manifest |

Not effects: allocation, panics, non-termination, reading immutable data, `spawn`/`async` themselves (the spawned
work's effects are the spawner's). `time`, `random` and `console` are *determinism* effects: harmless for
confinement, so granted by default, but visible to `durable fn`, replay and simulation. `fs`, `net`, `env`, `process`
and `ffi` are *authority* effects.

### 3. Inference by default, annotations optional (progressive disclosure)
**Surface rule: keywords change execution; attributes make claims.** A keyword (`async fn`, `durable fn`, `spawn`,
`scope`) changes how code runs: removing it changes behaviour. An attribute (`@uses`, `@pure`, `@effect`, `@derive`)
states a claim the compiler checks or a name it records: removing a *true* claim never changes behaviour, and a false
claim is a compile error. That is why `durable fn` is a keyword (it makes the runtime journal steps) and `@uses`/`@pure`
are attributes (they only assert what inference already computes).

- **Level 0, write code.** No annotations. `fs::read_to_string("cfg.toml")` performs `fs.read("cfg.toml")`; callers
  inherit it. `wj run` of a script in a directory with no manifest uses the *local profile* (decision D1).
- **Level 1, see it.** `wj explain-effects` prints each function's effect set and the call chain to every sink. `wj check`
  reports effects that the manifest does not grant.
- **Level 2, grant precisely.** `wj.toml [capabilities]` lists hosts, paths, names. Literal resource arguments are
  checked at compile time; computed ones get a runtime guard (the `grant_check` fact).
- **Level 3, contract.** `@uses(fs.read, net.connect("api.example.com"))` on a function (or `@pure` = `@uses()`) states its
  effects; the compiler checks the inferred set is inside the declaration. Required only at open-world boundaries:
  trait methods meant for downstream implementation, function-typed fields called inside `durable fn`, `extern`
  declarations. Optional everywhere else.
- **Level 4, capabilities as values.** `std::cap` handles (`Net`, `Fs`, `Env`, `Process`, `Clock`, `Rng`) attenuated with
  `net.only("api.example.com")`, `fs.under("./data").read_only()`, passed as ordinary parameters (section 6).
- **Level 5, durable and replayable.** `durable fn`, record/replay, simulation (sections 9 and 10).

### 4. Effect polymorphism, kept simple
- **Function parameters.** A call to a function-typed parameter does not add effects to the callee's own set; it adds
  `latent(fn, param)`. At each call site the argument's effects (a closure literal's body, or a named function's set)
  are added to the *caller*. So `xs.map(|x| fs::read_to_string(x))` makes the caller perform `fs.read`, and `map`
  stays pure. No syntax, no effect variables.
- **Escaping function values** (stored in a field, returned, sent on a channel): their effects are attributed to the
  site that **creates** the closure (lexical attribution, section 7). A later call through the stored value adds no
  effects to the calling function's set for authority purposes. For determinism purposes, a call through a value of
  function type whose effects are not declared is *effect-open*: inside a `durable fn` it is a journaled step like
  any other effectful call (section 9). A function type may state its effects: `fn(int) -> int @uses()`.
- **Traits and generics.** The program is compiled whole (the app and its Windjammer dependencies from source), so a
  `dyn Trait` or generic method call performs the union of the effects of the impls that exist in the program for the
  receiver types that reach the call (falling back to all impls of the method). A trait may declare effect bounds on a
  method (`@uses(...)` on the trait method); impls must stay inside it (checked), and then the bound is used instead
  of the union. Libraries that are published for downstream impls should declare bounds.
- **No effect handlers in the language in v1.** Handlers (in the algebraic-effects sense) exist only in the runtime
  port (section 10): recording, replay and simulation re-interpret the same sink calls. Algebraic effect handlers
  as surface syntax are explicitly out of scope until the port has shipped and been used.

### 5. Grants: the manifest, the CLI, the lock file
```toml
[capabilities]                       # grant for this package's own code
net.connect = ["api.example.com", "*.github.com:443"]
fs.read = ["./data/**", "./config.toml"]
fs.write = ["./out/**"]
env.read = ["DATABASE_URL"]
process = ["git"]
ffi = ["sqlite3"]
effects = ["ledger.post"]            # user-defined effects
# time, random, console are granted unless listed in deny
deny = []

[capabilities.dependencies.json5]    # per dependency (inferred sets are recorded in the lock file)
net.connect = []

[policy]
"ledger.post" = { only_via = ["billing"] }   # only module billing may perform it directly
forbid.ui = ["net", "fs.write"]              # module ui may not perform these, even transitively
```
- `wj run --interpret` CLI: `--sandbox` (empty grant: only `time`, `random`, `console`) plus `--allow net.connect=api.example.com`
  (repeatable; the same grammar as the manifest). `--allow-network` stays as an alias for `--allow net.connect=*`.
- **Per package.** Each Windjammer dependency's code runs under its own grant (lexical attribution, section 7). The lock
  file (WJ-SEC-03, `.wj-capabilities.lock`) records each dependency version's inferred effect set; an upgrade that
  adds an effect fails `wj build --locked` until `wj caps approve` records it. Stage E2, after ADR-0010.
- `[app_capabilities] allow/deny` (today) is read as a coarse grant with a deprecation warning and a `wj fix` rewrite.

### 6. Capabilities as values (`std::cap`)
The ambient std calls are the friendly default; capability values are for confinement *inside* a program:
```windjammer
use std::cap
fn fetch_all(net: cap::Net, urls: Vec<string>) -> Vec<string> { ... net.get(u) ... }
fn main() {
    let net = cap::net().only(["api.example.com"])     // attenuate the ambient grant (needs net.connect itself)
    fetch_all(net, urls)
}
```
- A capability value is an immutable, cheap, cloneable handle (ownership inference treats it like any value; no
  linearity in v1). Attenuation only narrows (`only`, `under`, `read_only`, `with_methods`); nothing widens.
- An operation through a value is checked against the value's scope (runtime guard unless proven). A function that
  only uses capability parameters has effects `via param` (shown by explain) and needs **no ambient grant** in its own
  package: that is how a dependency can do I/O for the app without being granted ambient authority (delegation).
- Code-mode and processes use the same values: ADR-0019 `spawn_process(f, s, caps: [...])` takes capability values;
  a child holds at most its parent's.

### 7. Attribution and checking
- **Lexical attribution.** An ambient sink call is checked against the grant of the package/module that contains the
  call site (the closure's creation site for closures), never the dynamic caller. A dependency cannot use the app's
  grant by being called from `main`, and a callback the app hands to a dependency keeps the app's grant.
- **Static check** (`wj check`, `wj build`): for every function, `effects(fn)` (direct sinks plus callees) must be inside
  its package grant; literal resources must match the refinement (`grant_check = Proven` or `Denied`). Policy rules are
  checked over provenance: `only_via` restricts which modules contain a *direct* sink call of that effect; `forbid`
  restricts a module's transitive set.
- **Dynamic check:** each `Runtime(scope)` site gets a guard. Path scopes are checked on the canonicalized path, opened
  relative to the scope root (no `..`, no symlink escape; TOCTOU-safe open on Rust via `openat`-style APIs); host scopes
  are checked on the URL host after parsing and again on every redirect hop; IP literals and private ranges are denied
  unless granted literally.
- **Denial is a fault, not an error value** (decision D4): a denied capability stops the program with a
  `capability denied` runtime error (exit status 126) naming the effect, the resource, the grant in force and the flag
  or manifest line that would allow it. It is not catchable by the program.

### 8. Backends
| Backend | Static facts | Runtime |
|---|---|---|
| Interpreter | reads the facts for diagnostics only | enforces **every** sink call dynamically (it is the sandbox), records to the effect log in record mode; ignores `Proven` (defense in depth) |
| Rust | `Proven` sites emit a plain std call (zero cost); `Denied` is a compile error | `Runtime(scope)` sites call a guard in `windjammer-runtime` (`caps::check_path`, `caps::check_host`) |
| Go / JS | same facts | guards in the portable runtime wrappers; JS in the browser: `net.connect` guard only, fs absent |

Compiled binaries are confined only as far as their code is Windjammer: `extern` code and Rust crates run with the
process's OS authority (section 11). The sandbox for untrusted code is the interpreter (and later the WASM target).

### 9. `durable fn`
```windjammer
durable fn checkout(order: Order) -> Result<Receipt, string> {
    let quote = pricing::quote(order.items)?          // effectful call: journaled step
    if time::now() > order.deadline { return Err("expired") }   // journaled
    step(retry: 3) { payments::charge(order.card, quote.total) }  // explicit step
}
```
Semantics: a durable function's progress survives crashes. Its arguments, each step result and its return value are
written to a journal through the ADR-0019 `DurableStore` port. After a crash the function re-runs from the start;
completed steps return their journaled result without executing (replay); the first incomplete step executes.

Rules (all compile-time, reported by `wj check`):
- **DUR-STEP.** Every call made from the durable body whose callee has a non-empty effect set (or is effect-open) is a
  step: it is journaled as a unit (`journal(site)`). An explicit `step { }` block groups several calls. So no ambient
  time, randomness or I/O happens outside a recorded step **by construction**; the user writes ordinary code.
- **DUR-VALUE.** A step result, the arguments and the return value must be journalable (types with a stable
  serialization: primitives, strings, collections and structs/enums of journalable types). A step returning a file
  handle or socket is an error: "do the I/O inside the step and return data".
- **DUR-MUT.** A step must not mutate durable state: an effectful call that takes a binding from the durable body by
  `&mut` (ownership fact `param_mode = MutRef`) is an error ("return the new value from the step instead"), because
  replay would skip the mutation.
- **DUR-SHARED.** The durable body must not read state it does not own: globals with interior mutability, `Shared`/
  `Mutex`/atomics, channel receives from outside, `select`.
- **DUR-ORDER.** Iterating a `HashMap`/`HashSet` in the durable body (outside a step) is an error, because their order is
  unspecified (as in Rust). The fix suggestion is `OrderedMap`/`OrderedSet` (decision D6), `BTreeMap`, or sorting first.
- **DUR-CALL.** Calling a `durable fn` from a durable fn is a child step (journaled with its own journal); calling one
  from ordinary code starts or resumes it by its identity (`durable::start(id, f, args)`; API in stage E4).
Delivery is at-least-once per step (a crash between performing and journaling re-executes the step); steps that must
not repeat use an idempotency key (`step.key()`). Journal format = the effect log (section 10) restricted to steps.

### 10. Record/replay and deterministic simulation
- **One port.** Every sink call goes through `perform(effect, op, args) -> result`. Implementations: `Live`, `Record`
  (live + append), `Replay` (answer from the log, verify the request), `Simulate` (seeded virtual world).
- **Log format** (JSON Lines, version 1): a header `{"wj_effect_log":1, "program": <source digest>, "compiler": <version>,
  "sched_seed": N, "sim_seed": N|null, "grant": {...}}`, then one event per line
  `{"seq":N, "task":T, "effect":"net.connect", "op":"http.get", "args":{...}|{"sha256":...}, "result":{...}|{"sha256":...}, "vt_ns":N}`.
  Large payloads go to a content-addressed side directory. Canonical JSON (sorted keys) so logs diff cleanly.
- **Replay** re-runs the program with the same scheduler seed and answers sinks from the log; a request that differs
  from the logged one at `seq` N is a **divergence**, reported with both requests and the source location.
  Because the scheduler is deterministic given the seed and the effect results, the seed plus the effect results
  determine the run; no schedule trace is needed in the interpreter.
- **Simulation** (`wj test --simulate --seed N`, `--seeds 1..1000`): virtual clock (starts at
  Unix time 1_700_000_000 s, the epoch `--deterministic-time` already uses; advances only by sleeps/timeouts), seeded `random`, in-memory filesystem seeded from a
  fixture directory, network answered by a mock table or by in-process simulated services, seeded fault injection
  (connection errors, partial writes, slow responses, crash points for durable fns). A failing seed is printed and
  re-runs exactly.
- Compiled targets get the same port in `windjammer-runtime` behind a build flag (`wj build --record`); a normal build
  calls std directly (zero cost). Stage E7.

### 11. Externs, Rust crates and trust boundaries
- An `extern` function must declare its effects (`@uses(net.connect, fs.read)` or `@pure`) (ADR-0012 option A); an
  undeclared extern has effect `ffi(<library>)`, granted only by `ffi = ["<library>"]`. Declarations are trusted claims:
  the compiler cannot verify Rust code. The lock file records them per crate version, so a changed claim needs approval.
- `--sandbox` (interpreter) denies `ffi` entirely; the interpreter has no FFI today, which keeps it that way by policy.
- Rust crate build scripts and proc macros run at build time with the builder's authority: out of scope; recommend
  sandboxed builds and `cargo vet` in the security guide.

### 12. Code mode: `wj mcp bind`
- `wj mcp bind <server> --out bindings/github.wj` reads an MCP server's tool list (JSON Schema per tool) and generates
  a typed module: one struct/enum per schema, one function per tool returning `Result<T, ToolError>`, each annotated
  `@effect("mcp.github.create_issue")`. OpenAPI documents are a second input format later.
- Agents write Windjammer against the bindings. `wj check --json` gives typed errors before anything runs (the
  self-repair loop). `wj explain-effects --json script.wj` lists exactly which tools, hosts and paths the script can
  touch: the effect set **is** the permission request a host can show or auto-approve.
- `wj run --interpret --sandbox --bind github=<endpoint> --allow effects=mcp.github.list_issues,mcp.github.create_issue
  --limits fuel=1e8,mem=256MiB,wall=10s script.wj`: the tool calls go through the effect port to the host's MCP
  client, which holds the credentials; the script never sees tokens. Results: stdout, `--result-json`, and the effect
  log (audit and replay).
- `windjammer-mcp` gains an `execute(code, grant)` tool so any MCP client can use code mode with the sandbox.
- Resource limits (fuel = interpreter steps, memory, wall time, output bytes, number of effect calls) are part of
  the sandbox and are new interpreter work.

### 13. Prompt injection and information flow
Capabilities bound the blast radius: a script that reads private data and is not granted any outbound channel cannot
exfiltrate through Windjammer code (the "lethal trifecta" needs all three legs). They do not stop misuse of a
granted channel. The precise defence is information-flow control: values from untrusted sources (tool results marked
`untrusted` by the binding, network responses) carry a provenance label in the interpreter, and `[policy.flows]`
rules refuse flows such as `untrusted -> mcp.mail.send` (or require host approval). This extends the taint solver
(WJ-SEC-02) to the dynamic interpreter. Research stage E6; not promised by this ADR.

### 14. Threat model
Adversaries: (T1) untrusted or prompt-injected program run in the sandbox; (T2) malicious or compromised dependency;
(T3) honest but wrong application code (policy); (T4) hostile input data.
Trusted computing base: the interpreter, the sink table, the effect port and guards, manifest/CLI parsing, the OS.

| Escape route | Mitigation | Test family |
|---|---|---|
| path traversal, symlinks, `..`, case tricks, hard links | canonicalize + open beneath the scope root; deny on mismatch | `escape/fs/*` |
| host allowlist bypass: redirects, IP literals, DNS rebinding, userinfo `@` in URLs, IDN | parse with one URL parser; check host per hop; deny literal/private IPs unless granted; check the resolved address against private ranges | `escape/net/*` |
| writing its own manifest, `.wj` sources or the lock file | the manifest, lock file and sources are outside every fs.write grant unless named explicitly | `escape/self/*` |
| environment enumeration (`env::vars()`) | returns only granted names | `escape/env/*` |
| `process` grant | a granted program runs with OS authority: the grant is per program name and `explain` warns that it is ambient for the child | `escape/process/*` |
| `extern` / FFI, Rust crates | denied in the sandbox; declared-effects trust + lock file for builds | `escape/ffi/*` |
| resource exhaustion | fuel, memory, wall-time, output and effect-count limits | `escape/limits/*` |
| laundering via callbacks or capability values | lexical attribution; attenuation only narrows; values cannot be forged (no constructor outside `std::cap`) | `escape/attribution/*` |
| interpreter bugs | defense in depth: optional OS confinement (Landlock/seccomp on Linux, sandbox-exec on macOS) and the WASM target | fuzzing |
| timing and side channels | out of scope | |

Supply chain: per-package grants and the lock-file diff (T2); compile-time `forbid` policies (T3); IFC (T1/T4, stage E6).

### 15. Diagnostics and explain
Codes `WJ0040`-`WJ0049` are reserved for this ADR (WJ0024 stays as the coarse manifest violation and is retired into
WJ0040 when the new check lands):
`WJ0040` effect not granted; `WJ0041` resource outside the grant (literal host/path); `WJ0042` performs an effect its
`@uses` does not declare; `WJ0043` policy violation (`only_via`/`forbid`); `WJ0044` extern without declared effects;
`WJ0045` durable: non-journalable step value; `WJ0046` durable: step mutates durable state; `WJ0047` durable: shared
state or `select`; `WJ0048` durable: unordered iteration; `WJ0049` replay divergence (runtime).
Every diagnostic names the sink, the call chain from the function to it (from provenance), the grant in force and a
machine-applicable fix (the manifest line, or the `--allow` flag). `wj explain-effects [--fn NAME] [--json]` prints per
function: the effect set (compact form `main: console, fs.read("cfg.toml")`), each effect's chain and rule ID, the
grant status and the check placement (`proven` / `runtime guard` / `denied`).

## Options considered
- **A. Pure object capabilities (no ambient authority at all).** Strongest; every I/O needs a passed value. Rejected
  as the default: it fails the "humans love it" test for scripts and makes every example longer. Kept as level 4.
- **B. Explicit effect annotations required (Koka/Java checked exceptions).** Rejected: annotation burden, churn when
  a leaf gains an effect. Kept as optional level 3 and required only at open-world boundaries.
- **C. Runtime-only permissions (Deno).** Rejected alone: no compile-time feedback, no determinism proofs. Kept as the
  interpreter's enforcement.
- **D. Chosen:** inferred effects as semantic facts + ambient-at-the-root grants (manifest/CLI) + capability values for
  confinement + static proof where possible and runtime guards elsewhere.

## Staged plan (each stage: gate to enter the next; kill criteria)
| Stage | Content | Prerequisites | Gate (measured) | Kill / rethink if |
|---|---|---|---|---|
| E0 (done with this ADR) | ADR, spec chapter `effects.md`, xfail examples `tests/spec/effects/`, harness modes | - | `scripts/spec_check.sh` green; every EFF rule has an example | - |
| E1 MVP | sink table (incl. `time`, `random`, `console`); effect pass with provenance + `latent`; `wj explain-effects`; `wj check` vs `[capabilities]` (coarse + literal host/path); interpreter capability context: `--sandbox`, `--allow`, per-host net, fs scopes, env names, process names; denial fault | convergence C2 (provenance); C3 (DefId) for the soundness gate | (1) soundness: on the conformance and examples corpus run in record mode, every performed effect is in the static set of the enclosing function (dynamic subset of static) in 100% of runs; (2) EFF-001..012, 018-021 examples pass; (3) escape suite v1 (fs, net, env, self, attribution; at least 60 cases): 0 escapes; (4) interpreter overhead of enforcement at most 5% on the interpreter benchmark | soundness below 100% after C3 (inference cannot be trusted; stop and fix resolution); false-positive effect errors on more than 2% of corpus programs with a correct manifest (vocabulary too fine: merge effects) |
| E2 compile-time enforcement (delivered 2026-10-19, see "E2 status") | `wj build` uses facts (`Proven`/`Runtime`/`Denied`); guards in `windjammer-runtime` and portable wrappers; `@uses`/`@pure`; `[policy]`; per-dependency grants and lock file; `std::cap` values | E1; ADR-0010 for dependencies | (1) zero guards emitted for literal-only programs (counted in emitted code); (2) Rust benchmark suite delta at most 1% with effects on versus off (statically proven programs); (3) Go/JS conformance parity for guard behaviour; (4) EFF-013..017 pass | guards cannot be placed from facts without backend-local decisions (would violate ADR-0017): redesign the fact |
| E3 record/replay + DST (interpreter) | effect port; log v1; `--record`, `--replay`; `wj test --simulate --seed/--seeds`; fault injection | E1 | (1) replay bit-exact on 100% of recorded corpus runs; (2) planted-bug study: 20 concurrency/timing bugs in sample apps, at least 80% found within 10 minutes of seeds; (3) record overhead at most 2x | replay below 100% because of hidden non-determinism the port cannot see (then find and route it, or narrow the claim) |
| E4 `durable fn` | parser; DUR-* checks; journal over `DurableStore` (in-memory, file); crash injection | E3; ADR-0019 D1 (store port) | (1) mutation study: inject clock/random/IO/global/HashMap-order non-determinism into 10 sample workflows (ported from public workflow samples): 100% caught at compile time or journaled; (2) crash at every step boundary: final state equals the crash-free run in 100% of cases; (3) DUR examples pass | more than 20% of sample workflows need an escape hatch (rules too strict), or journaling costs more than 2x per step over an in-memory store |
| E5 code mode | `wj mcp bind`; `--bind`; limits (fuel, memory, wall, output, effect count); `execute` tool in `windjammer-mcp` | E1 (can run in parallel with E2-E4) | (1) red-team suite of 50 prompt-injected scripts: 0 escapes; (2) sandbox cold start under 5 ms; (3) same-model comparison against TypeScript code mode on a public task set: task success at least equal at equal or fewer tokens | worse success at equal tokens: keep the sandbox, drop the code-mode positioning |
| E6 information flow | provenance labels in the interpreter; `[policy.flows]` | E5 | an AgentDojo-style subset: attack success near 0 with utility within 10 points of undefended | utility loss above 20 points |
| E7 compiled record/replay, OS confinement | runtime port behind `--record`; Landlock/seccomp/sandbox-exec option | E2, E3 | replay parity with the interpreter on the conformance corpus | - |

### E1 status (2026-10-19): implemented, gates measured
Where: sink table, grants and the effect pass in `src/ir/effect_facts/`; findings in `src/check/effects.rs` (WJ0040-WJ0044,
WJ0024 stays the legacy coarse code); interpreter gate in `src/interpreter/caps.rs` (one choke point, `call_builtin_fn`, plus
host-bridge externs); `wj explain-effects`; `wj run --sandbox/--allow/--caps`. Acceptance tests: `docs/spec/effects.md`
(SPEC-EFF-001..011, 013..016, 018..020, 027, 028 pass; 012, 021..026, 029 stay `xfail Q-032` for E2-E4).

| E1 gate | Result (evidence) |
|---|---|
| Soundness: dynamic effects within the static set of `main` | 0 violations over the whole corpus (the spec examples plus the effectful ones in `tests/effects/corpus/`); `tests/effects_soundness_test.rs`. `console` was static-only in E1 (the `println` intrinsic was not a gated builtin); E2 observes it too |
| Escape suite, at least 60 cases, 0 escapes | 107 cases, 0 escapes (`tests/effects_escape_test.rs`): fs (dotdot, absolute, dir and file symlinks, sibling prefix, info leaks via exists/metadata, rename/copy across the boundary, manifest and source protection), net (userinfo, fragment, query, backslash, suffix, wildcard apex, port, scheme default, IP/IPv6/localhost, trailing dot, case, every verb, listen), env, process (shell metacharacters, local-path borrowing a granted name), time/random/console revocation, closures, spawned tasks, uncatchable denial |
| Interpreter overhead at most 5% | non-sink-heavy workload: +3.8% CPU (median of 18 interleaved runs each, output identical); **provisional, measured at load 57-65**. Sink-dense loop (`fs::exists` + `random` + `time` x 30000): about 22 microseconds per fs check (path canonicalization is a few syscalls), 6x on that microbenchmark; re-measure on a quiet machine |
| False-positive effect errors at most 2% of the corpus with a correct manifest | none in the corpus: no program's static set names an effect kind its run never exercised (`console` aside) |

Decisions taken while implementing (all enforced by tests): one URL parser (`scope_match::split_url`) serves both the check and
the interpreter's http client, so what is checked is what is connected to; `process::run` programs are resolved by
`parse_command` (shell syntax means `sh`, and a bare name never matches a local `./name`); a glob grant never covers
`wj.toml`, `*.wj-capabilities.lock` or `*.wj` (only an exact pattern does); `fs::set_current_dir` is `env.write(cwd)`.
Known limits: TOCTOU between check and open; DNS rebinding (hosts are checked by name); redirects (the interpreter
client does not follow any); per-package grants and `std::cap` values are E2; `console` is not gated at run time.

### E2 status (2026-10-19): implemented, gates measured
Where: per-package grants and the lock file in `src/ir/effect_facts/{packages,lock}.rs` (`wj caps show|approve`, `src/cli/caps.rs`);
dependency sources for the pass in `src/check/dep_sources.rs`; `std::cap` values in `src/interpreter/builtins/std_cap.rs` and
`crates/windjammer-runtime/src/cap.rs` (declared in `std/cap.wj`); build-time enforcement and grant installation in
`src/cli/build_caps.rs` + `src/ir/effect_facts/install.rs`; run-time guards in `crates/windjammer-runtime/src/caps.rs` (Rust),
`std/runtime/js/wj_std.mjs` and `std/runtime/go/wj_std.go`.

Decisions taken while implementing (each enforced by a test):
* **One capability model, one source.** `crates/windjammer-runtime/src/caps_core/` (vocabulary, path/host/name matching, grants) is
  compiled into BOTH the compiler (`#[path]`, as `ir::effect_facts::model`) and the runtime, so the interpreter, `wj check` and compiled
  programs cannot disagree. Go and JavaScript cannot share Rust, so `tests/effects/vectors.json` (60 cases) is run against all three
  (`tests/effects_vectors_test.rs`). The compiler's packaging already ships the runtime sources; no new crate.
* **Proven means no code.** A call with a literal resource inside the grant, or a resource-less effect the grant allows, adds nothing: the
  emitted Rust of a proven program is byte-identical with and without `[capabilities]` (test). Only a computed path/host/name, or a
  capability value minted from a scoped grant, makes `main` start with `caps::install(..)` (`$caps_install` / `wjstd_caps_install`).
  The grant is installed with root `.`: relative patterns anchor at the program's working directory, like the paths it opens.
* **A compiled program is held to one grant at run time.** Per-package attribution needs the call stack, which compiled code does not
  keep, so a dependency's computed-resource use must have the same scope as the application's (or `true`), else the build stops (WJ0041)
  and says to align the entries or pass the dependency a capability value. Literal uses are still checked per package, statically.
* **Dependencies are denied by default once the app opts in** (a `[capabilities]` table or a lock file): a dependency's effects are
  allowed by `[capabilities.dependencies.NAME]` and by what `.wj-capabilities.lock` records; an upgrade that adds an effect is a
  WJ0040/WJ0041 naming the dependency until `wj caps approve`.
* **Delegation by capability value.** A package handed a `Net`/`Fs`/... needs no ambient grant of its own for what the value allows;
  minting needs the minting package's grant; narrowing can only shrink (`derive_scope`). A closure keeps the grant of the package that
  wrote it.
* **JavaScript denial is an exception, not `process.exit`**: programs run in a worker with top-level `await`, and `process.exit` there
  crashes V8; `$CapFault` propagates to `$run`, which exits with status 126; no program can catch it.
* Portable (Go/JS) programs can only reach `fs` and `time` today, so their guards cover `fs.read`/`fs.write`; `net`, `env`, `process`
  appear in those backends when their std modules do (the vectors already carry the cases).

| E2 gate | Result (evidence) |
|---|---|
| Zero guards for statically-proven programs | emitted Rust byte-identical with and without `[capabilities]` (`a_proven_program_compiles_to_the_same_rust_with_and_without_capabilities`); Rust delta is 0 by construction |
| Every run-time sink in the Rust runtime is guarded | `tests/effects_runtime_audit_test.rs` scans fs/http/env/process/subprocess/db against OS-touching tokens, with a mutation check on the scanner itself |
| Go/JS parity with the Rust model | 60 vectors, three implementations, one table (`tests/effects_vectors_test.rs`) |
| Compiled enforcement end to end | Rust program built with cargo and run: allowed read prints, escape exits 126 (`WJ_EFFECTS_COMPILED=1`); JS (node) and Go programs the same, in the normal run |
| Escape suite after E2 | 130 cases, 0 escapes (adds capability values, console revocation); soundness still holds with `console` observed, and 1 corpus entry has a static effect it never exercised (about 0.5%, under the 2% bar) |
| Per-package grants, lock file, escalation | `tests/effects_packages_test.rs` (7 cases, through the `wj` binary) |
| `std::cap` | SPEC-EFF-012, 034, 035 and 23 of the 130 escape cases (narrowing never widens; delegation; symlinks and `..` through `under`; process and env values) |
| Interpreter overhead at most 5% (re-measured at load below 20) | **Met: +3.5% CPU** on the non-sink workload (median of 10 interleaved runs per arm, identical output, load average 17.4 before and 19.2 after: `scripts/bench_effects_overhead.py`, which refuses a verdict above load 20). The sink-dense microbenchmark (`fs::exists` + `random` + `time`, 30k iterations) is about 6.6x slower, roughly 18 microseconds per file-system check, because every fs resource is canonicalized (a few syscalls) so symlinks cannot escape; it is outside the gate's scope (general programs) and reported so nobody is surprised |

The same default sandbox applies to `wj test --interpret` and the REPL (their messages offer only the `wj.toml` fix: no `--allow` there). A verified example declares what it needs with `// allow:` header lines; a mocked http port (`--mock-http`) needs no network capability in the default profile, but an explicit grant keeps governing mocked calls.

Not done in E2: `@uses` bounds on trait methods (the parser keeps no attributes on trait methods; SPEC-EFF-006 still uses the union of impls);
`console` is gated at run time in the interpreter only (compiled backends reject a denied `console` at compile time, which is exact because the
effect has no resource); Go/JS have no `net`/`env`/`process` std modules to guard yet.

## Measurable success criteria (whole feature)
- Soundness: dynamic effect trace is a subset of the inferred set, 100% of the corpus (CI gate from E1).
- Escape suite: at least 150 cases by E5, 0 escapes (CI gate).
- Zero cost: statically proven programs emit 0 guards; Rust runtime delta at most 1% (benchmarked).
- Zero config: at least 95% of `examples/` need no annotation, only manifest grants.
- Durable: 100% of injected non-determinism caught or journaled; 0 divergences under crash injection.
- Replay: 100% bit-exact on recorded runs.
- Agent loop: every effect diagnostic carries a machine-applicable fix (checked by the diagnostics suite).

## Consequences
- `src/ir/effects.rs` heuristics are replaced by the sink table; `Effect` gains `Time`, `Random`, `Console`, refinements and
  provenance. `[app_capabilities]` is deprecated in favour of `[capabilities]`.
- The interpreter's `HttpMode` becomes one case of the capability context; fs/env/process builtins gain checks.
- Std effect classification becomes a compatibility surface (changing a std function's effect is a breaking change).
- ADR-0009's choice (per-app now, per-dependency later, `unknown` for externs) is implemented by E1/E2.
- WJ-SEC-01..04 remain as background; where they disagree with this ADR, this ADR wins.

## Decisions (project owner, 2026-10-19)
Follow-up answers (2026-10-19, after E1): the local profile was staged as warn-then-enforce at v0.51.0, and the owner then decided to
**enforce it from this release on** (the staging is dropped): `gate_for_run` reads one constant, `LOCAL_PROFILE_MODE = Enforce`, and
`WJ_CAPS=warn|off` (`--caps`) stay as explicit opt-outs; a denial in the local profile prints why, the one-shot `--allow` flag and the
copy-paste `[capabilities]` block (including the project-directory `fs` lines, because a `[capabilities]` table is the whole grant);
standard devices are not files (`/dev/stdin`, `/dev/stdout`, `/dev/stderr` are the console, `/dev/urandom` is randomness, `/dev/null`
is free), so reading stdin never needs a grant; the allowlist rebase was done by the integrator; **E2 gates `console` at run time** (granted in the local profile and by default, so it costs nothing, visible in
`explain-effects`, revocable for sandboxes); `ffi` stays denied in the local profile, the WJ0040/WJ0044 fix line names exactly the
`[capabilities] ffi = ["name"]` entry to add, and a test requires any `wj new` template that declares an `extern fn` to ship it.

D1-D5, D7 and D8 are accepted as recommended; D6 is decided as written below (2026-10-18). D1 is staged: the release
that ships E1 first ran the local profile in **warn** mode; as of the follow-up decision above it **enforces**. A manifest
`[capabilities]` table, `--sandbox` or `--allow` always enforced. `WJ_CAPS=warn` (a sink outside the profile runs and prints
`warning[WJ0040]` once) and `WJ_CAPS=off` are the opt-outs.
- **D1. Default grant without a manifest** for `wj run --interpret`. Recommended *local profile*: fs read/write inside
  the project directory, `time`, `random`, `console`; net/env/process/ffi denied with a message naming `--allow`.
  Alternative: keep today's ambient fs/env/process (compatible, weaker).
- **D2. Annotation surface:** `@uses(...)`/`@pure`/`@effect("...")` attributes (recommended: no new keywords, matches
  `@derive`), or a `uses` clause after the return type.
- **D3. `durable fn` keyword** (recommended, as briefed) or `@durable` attribute.
- **D4. Capability denial is an uncatchable fault (exit 126)** (recommended) or a catchable error value.
- **D5. Default grants for `time`, `random`, `console`** (recommended: granted; visible to durable/replay only).
- **D6. HashMap/HashSet iteration order (decided 2026-10-18):** `HashMap`/`HashSet` follow Rust conventions (unspecified
  order, randomized hashing). Insertion-ordered maps and sets are separate opt-in types, `OrderedMap`/`OrderedSet`
  (indexmap on Rust, `Map` on JS, a small ordered table on Go), with `BTreeMap` for sorted order. DUR-ORDER stays. The
  interpreter keeps a deterministic iteration order by default and offers `--hash-seed N` to perturb it so code that
  depends on iteration order is caught by a differential gate.
- **D7. Runtime guards in compiled release binaries** for non-literal resources: on by default (recommended) or only
  with `--sandbox` builds.
- **D8. Sequencing of code mode (E5)** right after E1, in parallel with E2-E4 (recommended), or after E4.
