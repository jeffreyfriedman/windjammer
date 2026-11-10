# Effects and capabilities
<!-- feature: effects -->

A function either computes a result or does something to the outside world: reads a file, opens a connection, looks
at an environment variable, starts a process, asks for the time. Windjammer calls those things **effects**. It infers
which effects each function performs, and it lets you decide which of them a program may perform at all.

The effects the language knows are `fs.read`, `fs.write`, `net.connect`, `net.listen`, `env.read`, `env.write`,
`process`, `time`, `random`, `console` and `ffi`, plus any you define yourself. Some carry a resource: a path, a host,
a variable name, a program name.

## See what a program does

`wj explain-effects` lists the effects of every function, and the call chain that leads to each one:

```sh,verify
$ cat > report.wj <<'EOF'
use std::fs

fn add(a: int, b: int) -> int {
    a + b
}

fn load() -> string {
    fs::read_to_string("config.txt").unwrap_or("")
}

fn main() {
    println("${add(1, 2)} ${load().len()}")
}
EOF
$ wj explain-effects report.wj
add: pure
load: fs.read
main: console, fs.read
main -> load -> fs::read_to_string
```

`add` is pure. `load` reads a file. `main` inherits that and also prints. Nothing was annotated: the effects come from
what the code calls, all the way down through the standard library.

## The default: your project, and nothing else

When you run a program, `wj run` allows files inside the project directory, plus time, random numbers and console
output. Anything else stops the program with exit status 126 and a message that already contains the fix:

```sh,verify
$ cat > tool.wj <<'EOF'
use std::env

fn main() {
    let home = env::var("HOME").unwrap_or("unknown")
    println("home is set: ${home != "unknown"}")
}
EOF
$ wj run tool.wj 2>&1 || true
capability denied: env.read(HOME)
to allow just this once: run with `--allow env.read=HOME`
[capabilities]
env.read = ["HOME"]
```

The message names the effect and the resource, the way to allow it once, and the lines to put in `wj.toml` to allow it
for good. A denial is a policy fault, not an error value: the program cannot catch it and carry on.

Allow it for one run with `--allow`:

```sh,verify
$ cat > tool.wj <<'EOF'
use std::env

fn main() {
    let home = env::var("HOME").unwrap_or("unknown")
    println("home is set: ${home != "unknown"}")
}
EOF
$ wj run --allow env.read=HOME tool.wj
home is set: true
```

## Grants in `wj.toml`

The `[capabilities]` table in the manifest is the whole grant for the project. Each key is an effect, and its value
lists the resources allowed:

```toml
[capabilities]
fs.read = ["./**"]
fs.write = ["./**"]
net.connect = ["api.example.com", "*.example.org"]
env.read = ["HOME"]
process = ["git"]
```

A table with no entries grants only time, random and console, the same as `wj run --sandbox`. Because the manifest is a
file in your repository, a review of a pull request is also a review of what the program may do.

## Checked before you run

`wj check` and `wj build` compare what your code does with what the manifest grants, and report the difference with
the whole call chain, so a surprising effect has an address:

```sh,verify
$ mkdir -p reader/src
$ cat > reader/wj.toml <<'EOF'
[package]
name = "reader"
version = "0.1.0"

[capabilities]
fs.read = ["./config/**"]
EOF
$ cat > reader/src/main.wj <<'EOF'
use std::fs

fn load() -> string {
    fs::read_to_string("/etc/hosts").unwrap_or("")
}

fn main() {
    println(load())
}
EOF
$ cd reader && wj check 2>&1 || true
WJ0041
main -> load -> fs::read_to_string
grant in force: fs.read = [./config/**]
```

The path is a literal, so the check can see it is outside the grant and says so at compile time, as `WJ0041`. When a
resource is computed at run time, the same check happens when the call is made.

The related codes are `WJ0040` (an effect the grant does not include at all), `WJ0042` (a function that does more than
its declaration says), `WJ0043` (a policy violation) and `WJ0044` (a foreign function with no declared effects). Each
has a full explanation: `wj explain WJ0042`.

## Say what a function may do

Annotate a function with `@pure`, or `@uses(...)` with the effects it is allowed, and the compiler holds it to that:

```sh,verify
$ cat > pure.wj <<'EOF'
use std::fs

@pure
fn parse(s: string) -> int {
    let extra = fs::read_to_string("x").unwrap_or("")
    s.len() + extra.len()
}

fn main() {
    println("${parse("abc")}")
}
EOF
$ wj check pure.wj 2>&1 || true
WJ0042
parse
fs.read
```

## Hand a helper only what it needs

A grant belongs to the project. A **capability value** lets you hand a single function a narrower slice of it. The
function receives a `cap::Net` that can reach one host, and nothing in its body can widen that:

```sh,verify
$ mkdir -p fetcher/src
$ cat > fetcher/wj.toml <<'EOF'
[package]
name = "fetcher"
version = "0.1.0"

[capabilities]
net.connect = ["*"]
EOF
$ cat > fetcher/src/main.wj <<'EOF'
use std::cap

fn fetch(net: cap::Net, url: string) -> bool {
    net.get(url).is_ok()
}

fn main() {
    let api = cap::net().only(["api.example.com"])
    println("${fetch(api, "http://other.example/")}")
}
EOF
$ cd fetcher && wj run 2>&1 || true
capability denied: net.connect(other.example)
```

Even though the project may reach any host, `fetch` was given a value that reaches one, and the request to another host
is refused. `cap::fs().under(path).read_only()` does the same for files. Use these when you pass work to code you did
not write.

## Dependencies get their own grants

A package you depend on is checked against its own grant, not yours. `wj caps show` lists what each dependency is
known to do and whether the lock file covers it, and `wj caps approve` records the effects you have reviewed. Adding a
dependency that suddenly wants the network shows up as a change you can see.

## The opt-outs, spelled out

While you move an existing project onto grants, `WJ_CAPS=warn` runs every call and prints a warning where a denial
would have happened, and `WJ_CAPS=off` turns the gate off. Both are explicit, visible in the environment, and meant as
stepping stones:

```sh,verify
$ cat > tool.wj <<'EOF'
use std::env

fn main() {
    let home = env::var("HOME").unwrap_or("unknown")
    println("home is set: ${home != "unknown"}")
}
EOF
$ WJ_CAPS=warn wj run tool.wj 2>&1
home is set: true
```

## Going further

The interpreter can also run a program in a **simulated world** with a virtual clock, seeded random numbers and
in-memory files (`wj run --simulate --seed N`), record every effect of a run and replay it later (`--record`,
`--replay`), and journal a `durable fn` so a crashed run resumes where it stopped. Those features are specified in the
repository under `docs/spec/effects.md`; [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) lists what is still staged.
