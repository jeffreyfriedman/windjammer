# The effects model

An **effect** is something a function may do besides computing: `fs.read`, `fs.write`, `net.connect`, `net.listen`,
`env.read`, `env.write`, `process`, `time`, `random`, `console`, `ffi`, or an effect you define with
`@effect("name")`. An effect may carry a **resource**: a path, a host, a variable name, a program.

## Inference

| Rule | Behaviour |
|---|---|
| pure | a function that calls no sink, directly or through callees, performs no effect |
| sinks | each standard-library sink performs one effect: file reads `fs.read`, writes `fs.write`, the http client and TCP connect `net.connect`, servers `net.listen`, environment `env.read` and `env.write`, running a program `process`, clocks and sleeps `time`, randomness `random`, printing and reading stdin `console` |
| propagation | a function performs the union of its own sinks and its callees' effects |
| closures | a call through a function value counts as the effects the value is known to have |

`wj explain-effects FILE` prints the effects of every function with the call chain to each sink.

## Grants

A **grant** is the set of effects, with resource scopes, a package may perform. It comes from the `[capabilities]` table of
`wj.toml`, or from `--allow EFFECT[=RESOURCE]` for one run.

| Rule | Behaviour |
|---|---|
| default | with no `[capabilities]`, a run may use files inside the project, plus time, random and console |
| `time`, `random`, `console` | granted unless the manifest denies them; an empty `[capabilities]` table still allows them |
| denied call | stops the program with exit status 126 and a message naming the effect, the resource, the grant in force and the line that would allow it; it cannot be caught |
| `net.connect` | lists hosts, wildcard subdomains and optional ports; checked after URL parsing and again on every redirect |
| `fs.read`, `fs.write` | path scopes relative to the project root, checked after normalisation |
| `--sandbox` | start from an empty grant |
| `WJ_CAPS=warn` or `off` | explicit opt-outs while migrating |

## Static checks

| Code | Meaning |
|---|---|
| `WJ0040` | a function performs an effect the grant does not include |
| `WJ0041` | a literal resource is outside the grant |
| `WJ0042` | a function performs more than its `@pure` or `@uses` declaration says |
| `WJ0043` | a policy in `[policy]` forbids the call |
| `WJ0044` | an `extern fn` without a declared effect performs `ffi` and is not granted |

Every effect diagnostic shows the call chain from the reporting function to the sink and the manifest line that fixes it.

## Declarations and capability values

`@pure` declares no effects and `@uses(...)` declares a set; a function whose inferred effects exceed the declaration is
`WJ0042`. A `std::cap` value narrows a grant for one function: `cap::net().only(["host"])`, `cap::fs().under(path).read_only()`.
Attenuation never widens, and capability values cannot be built outside `std::cap`.

```sh,verify
$ cat > pure.wj <<'EOF'
@pure
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    println("${add(1, 2)}")
}
EOF
$ wj explain-effects pure.wj
add: pure
main: console
```

The [tutorial chapter](../tutorial/effects.md) walks through each of these with runnable commands.
