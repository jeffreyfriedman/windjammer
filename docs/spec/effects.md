# Effects and capabilities

[ADR-0021](../adr/0021-effects-and-capabilities.md) (accepted 2026-10-12; implementation staged E1-E7) decides
this chapter. **Stages E1 and E2 (interpreter side) are implemented:** effect inference (SPEC-EFF-001..007), grants enforced in the interpreter
(008..011), `wj check` findings (013..016, 018..020), `std::cap` values (012), per-package grants and the lock file (030), and `wj explain-effects`. Rules marked [PLANNED] carry an `xfail`
example: the example states the intended behaviour, it is expected to fail today, and the run fails if it starts
passing. They are the acceptance tests of the later stages (E2 compiled-backend guards; E3 replay;
E4 `durable fn`; E5 code mode). `--deterministic-time` and `--sched-seed` give a virtual clock, a seeded random
generator and reproducible interleavings (SPEC-EFF-027, SPEC-EFF-028).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's
executable example is the code block below it, stored in `tests/spec/effects/`. Example modes specific to this chapter
(see `tests/spec_conformance_test.rs`): `effects` (the compact `wj explain-effects` report), `check-ok`/`check-error`
(`wj check`, with the `manifest:` header lines as `wj.toml`), `simulate`, `replay`.

## Terms

An **effect** is something a function may do besides computing its result: `fs.read`, `fs.write`, `net.connect`,
`net.listen`, `env.read`, `env.write`, `process`, `time`, `random`, `console`, `ffi`, or a user-defined effect
(`@effect("ledger.post")`). An effect may carry a **resource** (a path, a host, a variable name, a program name).
A **sink** is a standard-library or `extern` function that performs an effect directly. A **grant** is the set of
effects (with resource scopes) a package may perform; it comes from `wj.toml [capabilities]` or the CLI. A **capability
value** is a `std::cap` handle that carries a narrowed grant. The compact effect report lists, per function, its
effects sorted by name, `pure` when there are none, `+ f` when it calls its function-typed parameter `f`, and `+ ?`
when it calls a function value whose effects are not declared.

## Inference

**SPEC-EFF-001** A function that calls no sink, directly or through callees, is pure; console output (`print`, `println`, `eprintln`, reading stdin) is the effect `console`.

<!-- example: tests/spec/effects/SPEC-EFF-001.wj -->
```windjammer
fn add(a: int, b: int) -> int { a + b }
fn main() {
    println("${add(1, 2)}")
}
```
`wj explain-effects` (compact):
```text
add: pure
main: console
```

**SPEC-EFF-002** Each standard-library sink performs one effect: file reads `fs.read`, file writes `fs.write`, the http client and TCP connect `net.connect`, servers `net.listen`, environment reads and writes `env.read`/`env.write`, running a program `process`, clocks and sleeps `time`, `std::random` and random UUIDs `random`. The table of sinks is part of the semantic layer and is the only classification the compiler and the interpreter use.

<!-- example: tests/spec/effects/SPEC-EFF-002.wj -->
```windjammer
use std::fs
use std::http
use std::env
use std::process
use std::time
use std::random
fn load() -> string { fs::read_to_string("cfg.toml").unwrap_or("") }
fn save(s: string) { let _ = fs::write("out.txt", s) }
fn fetch() -> bool { http::get("http://api.example.com/").is_ok() }
fn home() -> bool { env::var("HOME").is_ok() }
fn list() -> bool { process::run("ls").is_ok() }
fn stamp() -> int { time::now().timestamp() }
fn roll() -> int { random::range(1, 7) }
fn main() {}
```
`wj explain-effects` (compact):
```text
load: fs.read
save: fs.write
fetch: net.connect
home: env.read
list: process
stamp: time
roll: random
```

**SPEC-EFF-003** A function's effects are the union of its own sink calls and the effects of every function it calls (transitively, including recursion).

<!-- example: tests/spec/effects/SPEC-EFF-003.wj -->
```windjammer
use std::fs
use std::random
fn read_config() -> string { fs::read_to_string("cfg.toml").unwrap_or("") }
fn settings() -> string { "${read_config()}${random::range(0, 2)}" }
fn main() {
    println(settings())
}
```
`wj explain-effects` (compact):
```text
read_config: fs.read
settings: fs.read, random
main: console, fs.read, random
```

**SPEC-EFF-004** Calling a function-typed parameter adds no effects to the callee; the effects of the argument (a closure body or a named function) are added to the caller at the call site. `twice` and `apply` stay pure; `main` performs `random` because it passes a closure that does.

<!-- example: tests/spec/effects/SPEC-EFF-004.wj -->
```windjammer
use std::random
fn apply(f: fn(int) -> int, x: int) -> int { f(x) }
fn twice(f: fn(int) -> int, x: int) -> int { apply(f, apply(f, x)) }
fn main() {
    let a = twice(|x| x + 1, 0)
    let b = apply(|x| x + random::range(0, 1), 1)
    println("${a} ${b}")
}
```
`wj explain-effects` (compact):
```text
apply: pure + f
twice: pure + f
main: console, random
```

**SPEC-EFF-005** A closure's effects are attributed to the function that creates it. A call through a stored function value whose type declares no effects is *effect-open* (`+ ?`): it adds nothing to the calling function's effects for grants, and it is a journaled step inside a `durable fn` (SPEC-EFF-022).

<!-- example: tests/spec/effects/SPEC-EFF-005.wj -->
```windjammer
use std::random
struct Job { run: fn() -> int }
fn make() -> Job { Job { run: || random::range(0, 10) } }
fn run_job(j: Job) -> int { (j.run)() }
fn main() {
    println("${run_job(make()) >= 0}")
}
```
`wj explain-effects` (compact):
```text
make: random
run_job: pure + ?
```

**SPEC-EFF-006** A call to a trait method through a generic or `dyn` receiver performs the union of the effects of the impls of that method that reach the call in the program. A trait method may bound its impls' effects with `@uses(...)` or `@pure`; then the bound is used instead of the union (SPEC-EFF-041), and an impl that performs more than the bound is `WJ0042` (SPEC-EFF-042).

<!-- example: tests/spec/effects/SPEC-EFF-006.wj -->
```windjammer
use std::fs
trait Source { fn get(self) -> string }
struct FileSource { path: string }
struct Fixed { text: string }
impl Source for FileSource { fn get(self) -> string { fs::read_to_string(self.path).unwrap_or("") } }
impl Source for Fixed { fn get(self) -> string { self.text } }
fn read_any<S: Source>(s: S) -> string { s.get() }
fn main() {
    println(read_any(Fixed { text: "x" }))
    println(read_any(FileSource { path: "cfg.toml" }))
}
```
`wj explain-effects` (compact):
```text
read_any: fs.read
```

**SPEC-EFF-007** `spawn` and `async` (ADR-0004) are not effects; the effects of the spawned work are the spawning function's.

<!-- example: tests/spec/effects/SPEC-EFF-007.wj -->
```windjammer
use std::random
fn work() -> int { random::range(0, 5) }
fn main() {
    let t = spawn work()
    println("${t.wait() < 5}")
}
```
`wj explain-effects` (compact):
```text
work: random
main: console, random
```

## Grants and enforcement in the interpreter

**SPEC-EFF-008** `time`, `random` and `console` are granted unless the manifest denies them; a program with an empty `[capabilities]` table (the same grant as `wj run --interpret --sandbox`) can still print, read the clock and draw random numbers.

<!-- example: tests/spec/effects/SPEC-EFF-008.wj -->
```windjammer
use std::random
use std::time
fn main() {
    let r = random::range(0, 10)
    println("${r >= 0 && r < 10}")
    println("${time::now().timestamp() > 0}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
```
Output:
```text
true
true
```

**SPEC-EFF-009** When the grant in force does not include an effect, the sink call stops the program with a `capability denied` fault (exit status 126; not catchable). The message names the effect, the resource, the grant in force and the manifest line or `--allow` flag that would allow it.

<!-- example: tests/spec/effects/SPEC-EFF-009.wj -->
```windjammer
use std::fs
fn main() {
    println("start")
    let text = fs::read_to_string("/etc/hosts")
    println("${text.is_ok()}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
```
Output:
```text
start
```
Then the run stops with exit status 126 and a message containing `capability denied: fs.read`.

**SPEC-EFF-010** `net.connect` grants list hosts (`api.example.com`), wildcard subdomains (`*.example.com`) and optional ports; a request to any other host is denied. Hosts are checked after URL parsing and again on every redirect; IP literals and private address ranges are denied unless granted literally.

<!-- example: tests/spec/effects/SPEC-EFF-010.wj -->
```windjammer
use std::http
fn main() {
    let r = http::get("http://other.example/data")
    println("${r.is_ok()}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
net.connect = ["api.example.com"]
```
Then the run stops with exit status 126 and a message containing `capability denied: net.connect(other.example)`.

**SPEC-EFF-011** `fs.read` and `fs.write` grants list path scopes relative to the project root (`./data/**`). A path is checked after normalization (`..` and symlinks resolved), so a path that leaves the scope is denied and reported in its normalized form.

<!-- example: tests/spec/effects/SPEC-EFF-011.wj -->
```windjammer
use std::fs
fn main() {
    let r = fs::read_to_string("./data/../secret.txt")
    println("${r.is_ok()}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
fs.read = ["./data/**"]
```
Then the run stops with exit status 126 and a message containing `capability denied: fs.read(./secret.txt)`.

**SPEC-EFF-012** `std::cap` capability values narrow a grant (`cap::net().only([...])`, `cap::fs().under(path).read_only()`); an operation through a value is checked against the value's scope. Attenuation never widens a grant, and capability values cannot be constructed outside `std::cap`.

<!-- example: tests/spec/effects/SPEC-EFF-012.wj -->
```windjammer
use std::cap
fn fetch(net: cap::Net, url: string) -> bool { net.get(url).is_ok() }
fn main() {
    let api = cap::net().only(["api.example.com"])
    println("${fetch(api, "http://other.example/")}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
net.connect = ["*"]
```
Then the run stops with exit status 126 and a message containing `capability denied: net.connect(other.example)`.

## Static checks (`wj check`, `wj build`)

**SPEC-EFF-013** `wj check` reports `WJ0040` when a function performs an effect that its package's grant does not include.

<!-- example: tests/spec/effects/SPEC-EFF-013.wj -->
```windjammer
use std::process
fn main() {
    println("${process::run("ls").is_ok()}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
```
`wj check` reports an error containing: `WJ0040: effect `process` is not granted`

**SPEC-EFF-014** Every effect diagnostic shows the call chain from the reporting function to the sink (`main -> load -> fs::read_to_string`) and a machine-applicable fix (the manifest line).

<!-- example: tests/spec/effects/SPEC-EFF-014.wj -->
```windjammer
use std::fs
fn load() -> string { fs::read_to_string("/etc/passwd").unwrap_or("") }
fn main() {
    println(load())
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
fs.read = ["./config/**"]
```
`wj check` reports an error containing: `main -> load -> fs::read_to_string`

**SPEC-EFF-015** A literal resource outside the grant (a host, a path) is reported at compile time as `WJ0041`; a resource computed at run time gets a runtime check instead.

<!-- example: tests/spec/effects/SPEC-EFF-015.wj -->
```windjammer
use std::http
fn main() {
    println("${http::get("http://evil.example/").is_ok()}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
net.connect = ["api.example.com"]
```
`wj check` reports an error containing: `WJ0041: `net.connect(evil.example)` is outside the grant`

**SPEC-EFF-016** `@uses(effects...)` declares a function's effects and `@pure` declares none; a function whose inferred effects exceed its declaration is `WJ0042`.

<!-- example: tests/spec/effects/SPEC-EFF-016.wj -->
```windjammer
use std::fs
@pure
fn parse(s: string) -> int {
    let extra = fs::read_to_string("x").unwrap_or("")
    s.len() + extra.len()
}
fn main() {
    println("${parse("abc")}")
}
```
`wj check` reports an error containing: `WJ0042: `parse` performs `fs.read``

**SPEC-EFF-017** A function whose inferred effects are inside its `@uses` declaration and the grant is accepted.

<!-- example: tests/spec/effects/SPEC-EFF-017.wj -->
```windjammer
use std::fs
@uses(fs.read)
fn load(path: string) -> string { fs::read_to_string(path).unwrap_or("") }
fn main() {
    println(load("cfg.toml"))
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
fs.read = ["./**"]
```
`wj check` reports no error.

**SPEC-EFF-018** `[policy] forbid.<module> = [effects]` forbids a module's functions from performing those effects, directly or through callees (`WJ0043`).

<!-- example: tests/spec/effects/SPEC-EFF-018.wj -->
```windjammer
use std::http
mod data {
    pub fn latest() -> bool { http::get("http://api.example.com/").is_ok() }
}
mod ui {
    pub fn render() -> string { if super::data::latest() { "fresh" } else { "stale" } }
}
fn main() {
    println(ui::render())
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
net.connect = ["*"]
[policy]
forbid.ui = ["net"]
```
`wj check` reports an error containing: `WJ0043: module `ui` may not perform `net.connect``

**SPEC-EFF-019** `@effect("name")` declares a user-defined effect performed by calling that function. `[policy] "name" = { only_via = [modules] }` allows direct calls of its sinks only from the listed modules (`WJ0043`).

<!-- example: tests/spec/effects/SPEC-EFF-019.wj -->
```windjammer
mod ledger {
    @effect("ledger.post")
    pub fn post(amount: int) { println("posted ${amount}") }
}
mod billing {
    pub fn charge(amount: int) { super::ledger::post(amount) }
}
mod reports {
    pub fn adjust(amount: int) { super::ledger::post(amount) }
}
fn main() {
    billing::charge(5)
    reports::adjust(1)
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
effects = ["ledger.post"]
[policy]
"ledger.post" = { only_via = ["billing"] }
```
`wj check` reports an error containing: `WJ0043: `ledger.post` may only be performed via `billing``

**SPEC-EFF-020** An `extern` function must declare its effects with `@uses(...)` or `@pure`; without a declaration it performs `ffi`, which is granted only by name (`ffi = ["lib"]`), and `wj check` reports `WJ0044` when it is not granted.

<!-- example: tests/spec/effects/SPEC-EFF-020.wj -->
```windjammer
extern fn native_open(path: string) -> int
fn main() {
    println("${native_open("db")}")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
```
`wj check` reports an error containing: `WJ0044: extern `native_open` has no declared effects`

**SPEC-EFF-030** An ambient sink call is checked against the grant of the package that contains the call site (for a closure, where the closure is written), never the grant of the dynamic caller. (untestable: needs a multi-package fixture; acceptance test `tests/effects_packages_test.rs`)

**SPEC-EFF-033** `console` is an effect like the others: `print`, `println`, `eprint`, `eprintln`, stdin reads and `log::*` need it. It is granted by default and costs nothing; a sandbox can revoke it (`deny = ["console"]`), and a denied print stops the run with exit status 126.

<!-- example: tests/spec/effects/SPEC-EFF-033.wj -->
```windjammer
fn main() {
    println("this line is never printed")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
deny = ["console"]
```
The run stops with exit status 126 and a message containing `capability denied: console`.

**SPEC-EFF-034** A capability value can only be narrowed. `only(..)` on a `Net` accepts patterns the value already covers; asking for more is a `capability denied` fault, so a function handed `api` can never reach `other.example` through it.

<!-- example: tests/spec/effects/SPEC-EFF-034.wj -->
```windjammer
use std::cap
fn main() {
    let api = cap::net().only(["api.example.com"])
    let wider = api.only(["other.example"])
    println("not reached")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
net.connect = ["api.example.com", "other.example"]
```
The run stops with exit status 126 and a message containing `capability denied: net.connect(other.example)`.

**SPEC-EFF-035** `Fs` values narrow by directory (`under(dir)`) and by direction (`read_only()`); a write through a read-only value is denied even when the ambient grant allows it.

<!-- example: tests/spec/effects/SPEC-EFF-035.wj -->
```windjammer
use std::cap
fn main() {
    let files = cap::fs().under("/etc").read_only()
    println("start")
    let r = files.write("/etc/wj_never_written", "x")
    println("not reached")
}
```
Manifest (`wj.toml`):
```toml
[capabilities]
fs.read = ["/etc/**"]
fs.write = ["/etc/**"]
```
Output:
```text
start
```
Then the run stops with exit status 126 and a message containing `capability denied: fs.write(/etc/wj_never_written)`.

**SPEC-EFF-036** Dependencies are checked against their own grants (`[capabilities.dependencies.NAME]`) and against what `.wj-capabilities.lock` records for them; a dependency upgrade that adds an effect is reported (WJ0040/WJ0041 naming the dependency) until `wj caps approve` records it, and `wj caps show` lists each dependency's inferred effects. (untestable: needs a multi-package fixture; acceptance test `tests/effects_packages_test.rs`)

**SPEC-EFF-037** Compile-time enforcement: with a `[capabilities]` table, `wj build` fails on the same findings `wj check` reports (WJ0040-WJ0044) before any code is generated, for every backend. A call the compiler can prove inside the grant (a literal path, host or name) carries no check: the generated code is byte-identical with and without the table. (untestable: needs a compiled backend; acceptance tests `tests/effects_compiled_test.rs`)

**SPEC-EFF-038** Run-time guards: when some call cannot be proven (a computed path, host or name, or a capability value minted from a scoped grant), the generated program installs its grant as the first statement of `main` (`windjammer_runtime::caps::install` on Rust, `$caps_install` on JavaScript, `wjstd_caps_install` on Go) and the runtime's std wrappers check against it; a denial exits with status 126 and cannot be caught. Compiled programs hold the whole program to one grant, so a dependency's computed-resource use needs the same scope as the application's (or `true`), else the build says so. (untestable: needs a compiled backend; acceptance tests `tests/effects_compiled_test.rs`, `tests/effects_vectors_test.rs`)

**SPEC-EFF-039** Without a `[capabilities]` table (and without `--sandbox` or `--allow`) `wj run` enforces the local profile: files inside the project directory, plus `time`, `random` and `console`. Anything else stops the run with exit status 126; the message says why, offers the one-shot `--allow` flag and a copy-paste `[capabilities]` block that keeps the project-directory lines. `WJ_CAPS=warn` (run it, warn once) and `WJ_CAPS=off` are the explicit opt-outs. `wj test --interpret` and the REPL run under the same default (a project's `[capabilities]` is the way to grant them more).

<!-- example: tests/spec/effects/SPEC-EFF-039.wj -->
```windjammer
use std::env
fn main() {
    println("start")
    let v = env::var("WJ_NEVER_SET")
    println("not reached")
}
```
Output:
```text
start
```
Then the run stops with exit status 126 and a message containing `capability denied: env.read(WJ_NEVER_SET)`.

**SPEC-EFF-040** Standard devices are not files: `/dev/stdin` and `/dev/stdout`, `/dev/stderr` are the console, `/dev/urandom` and `/dev/random` are randomness, `/dev/null` needs nothing. Reading standard input therefore never needs a file grant.

<!-- example: tests/spec/effects/SPEC-EFF-040.wj -->
```windjammer
use std::fs
fn main() {
    let there = fs::exists("/dev/stdin")
    println("checked")
}
```
Output:
```text
checked
```

## Durable functions

**SPEC-EFF-021** [PLANNED ADR-0021] `durable fn` declares a function whose progress survives crashes. When nothing fails it behaves exactly like the same function without `durable`.

<!-- example: tests/spec/effects/SPEC-EFF-021.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
durable fn total(xs: Vec<int>) -> int {
    let mut s = 0
    for x in xs {
        s += x
    }
    s
}
fn main() {
    println("${total([1, 2, 3])}")
}
```
Output:
```text
6
```
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

**SPEC-EFF-022** [PLANNED ADR-0021] Every call from a durable body to a function with effects (or an effect-open call) is a step: its result is journaled, and on re-execution after a crash a completed step returns its journaled result without running. `step { }` groups several calls into one step. Ordinary code with clock, random and file reads is therefore accepted.

<!-- example: tests/spec/effects/SPEC-EFF-022.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::time
use std::random
use std::fs
fn quote(n: int) -> int { n * 10 + random::range(0, 3) }
durable fn checkout(n: int) -> string {
    let price = quote(n)
    let at = time::now().timestamp()
    let note = fs::read_to_string("note.txt").unwrap_or("")
    "${price} ${at} ${note}"
}
fn main() {
    println(checkout(2))
}
```
`wj check` reports no error.
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

**SPEC-EFF-023** [PLANNED ADR-0021] A step's result, the durable function's arguments and its result must be journalable (primitives, strings, collections, and structs/enums of journalable types); otherwise `WJ0045`.

<!-- example: tests/spec/effects/SPEC-EFF-023.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::random
fn make_counter() -> fn() -> int {
    let n = random::range(0, 9)
    move || n
}
durable fn pick() -> int {
    let c = make_counter()
    c()
}
fn main() {
    println("${pick()}")
}
```
`wj check` reports an error containing: `WJ0045: step result of type `fn() -> int` cannot be journaled`
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

**SPEC-EFF-024** [PLANNED ADR-0021] A step may not mutate a binding of the durable body (a `&mut` argument, as inferred by ownership inference), because replay would skip the mutation (`WJ0046`).

<!-- example: tests/spec/effects/SPEC-EFF-024.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::random
fn add_noise(acc: Vec<int>) { acc.push(random::range(0, 9)) }
durable fn sample() -> int {
    let mut acc = []
    add_noise(acc)
    acc.len()
}
fn main() {
    println("${sample()}")
}
```
`wj check` reports an error containing: `WJ0046: step `add_noise` mutates `acc``
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

**SPEC-EFF-025** [PLANNED ADR-0021] A durable body may not read state it does not own: channel receives, `select`, shared cells, locks, atomics and mutable globals (`WJ0047`).

<!-- example: tests/spec/effects/SPEC-EFF-025.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::sync
durable fn consume(rx: sync::Receiver<int>) -> int {
    rx.recv()
}
fn main() {
    let (tx, rx) = sync::channel()
    tx.send(1)
    println("${consume(rx)}")
}
```
`wj check` reports an error containing: `WJ0047: durable fn `consume` receives from a channel`
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

**SPEC-EFF-026** [PLANNED ADR-0021] A durable body may not iterate a `HashMap` or `HashSet` outside a step, because the order is not stable across backends and runs (`WJ0048`). [ADR-0021 D6 decided: `HashMap` keeps Rust's unspecified order; use `OrderedMap`/`OrderedSet` or `BTreeMap` in durable code.]

<!-- example: tests/spec/effects/SPEC-EFF-026.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::collections::HashMap
durable fn first_key(m: HashMap<string, int>) -> string {
    let mut out = ""
    for (k, v) in m {
        if out == "" { out = k }
    }
    out
}
fn main() {
    let mut m = HashMap::new()
    m.insert("a", 1)
    println(first_key(m))
}
```
`wj check` reports an error containing: `WJ0048: iteration over `HashMap` has no stable order`
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Simulation and replay

**SPEC-EFF-027** In the simulated world the wall clock starts at Unix time 1700000000 and advances only through sleeps and timeouts (the interpreter's `--deterministic-time` clock).

<!-- example: tests/spec/effects/SPEC-EFF-027.wj -->
```windjammer
use std::time
fn main() {
    println("${time::now().timestamp()}")
    time::sleep_millis(2000)
    println("${time::now().timestamp()}")
}
```
Simulated with seed 7 (`wj test --simulate`; the spec harness runs it twice):
```text
1700000000
1700000002
```

**SPEC-EFF-028** A simulated run is a function of the program and the seed: the same seed gives the same random numbers and the same task interleaving.

<!-- example: tests/spec/effects/SPEC-EFF-028.wj -->
```windjammer
use std::random
use std::sync
fn main() {
    let (tx, rx) = sync::channel()
    scope {
        for i in 0..3 {
            let t = tx
            spawn {
                t.send(i * 10 + random::range(0, 10))
            }
        }
    }
    for _ in 0..3 {
        println("${rx.recv()}")
    }
}
```
Simulated twice with seed 42: both runs print the same three lines.

**SPEC-EFF-029** [PLANNED ADR-0021] A run recorded with `--record LOG` replays from the log with `--replay LOG`: every sink call is answered from the log, the program prints the same output, and no live effect is performed.

<!-- example: tests/spec/effects/SPEC-EFF-029.wj -->
<!-- ignore: known divergence Q-032; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::random
use std::time
fn main() {
    println("${random::range(0, 1000000)}")
    println("${time::now().timestamp()}")
}
```
Recorded, then replayed from the effect log: both runs print the same lines and replay reports no divergence.
> **Known divergence:** the implementation does not do this yet; see [Q-032](OPEN_QUESTIONS.md). The example is marked `xfail`.

**SPEC-EFF-031** [PLANNED ADR-0021] During replay, a sink call that differs from the logged request at the same sequence number stops the run with a replay divergence (`WJ0049`) that shows both requests and the source location. (untestable: needs a log recorded from a different program; acceptance test `tests/effect_replay_test.rs`, stage E3)

## Code mode

**SPEC-EFF-032** [PLANNED ADR-0021] `wj mcp bind` generates one Windjammer function per tool of an MCP server, with typed parameters and results and the user-defined effect `mcp.<server>.<tool>`; a sandboxed run may call exactly the tools its grant lists. (untestable: generated from a live tool schema; acceptance test `tests/mcp_bind_test.rs`, stage E5)

**SPEC-EFF-041** A call through a trait method that carries `@uses(...)` performs exactly that bound, whatever the impls in the program do. Callers see a stable effect even when an impl is added later.

<!-- example: tests/spec/effects/SPEC-EFF-041.wj -->
```windjammer
use std::fs
trait Source {
    @uses(fs.read)
    fn get(self) -> string
}
struct Fixed { text: string }
struct Cached { text: string }
impl Source for Fixed { fn get(self) -> string { self.text } }
impl Source for Cached { fn get(self) -> string { self.text } }
fn read_any<S: Source>(s: S) -> string { s.get() }
fn main() {
    println(read_any(Fixed { text: "x" }))
}
```
`wj explain-effects` (compact):
```text
read_any: fs.read
```

**SPEC-EFF-042** An impl method whose effects exceed its trait method's `@uses(...)` or `@pure` bound is reported as `WJ0042` on the impl method.

<!-- example: tests/spec/effects/SPEC-EFF-042.wj -->
```windjammer
use std::fs
trait Source {
    @pure
    fn get(self) -> string
}
struct FileSource { path: string }
impl Source for FileSource { fn get(self) -> string { fs::read_to_string(self.path).unwrap_or("") } }
fn main() {
    println(FileSource { path: "cfg.toml" }.get())
}
```
`wj check` reports an error containing: `as Source>::get` performs `fs.read``
