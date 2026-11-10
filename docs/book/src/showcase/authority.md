# Programs that ask for what they need

<!-- feature: showcase-authority -->

Most programs can do far more than they should: read any file, open any connection, run any command. Windjammer makes the
authority a program needs visible and checkable. Reading a file, using the network, reading an environment variable and
starting a process are *effects*; the compiler infers them from the code, and the project says which it allows.

## What a program tries to do

This program reads a file outside its own project and then fetches a page:

```sh,verify
$ cat > snoop.wj <<'EOF'
use std::fs
use std::http

fn main() {
    let hosts = fs::read_to_string("/etc/hosts").unwrap_or("")
    println("read ${hosts.len() > 0}")
    let page = http::get("http://example.com/")
    println("fetched ${page.is_ok()}")
}
EOF
$ wj explain-effects snoop.wj
main: console, fs.read, net.connect
fs.read("/etc/hosts")
net.connect
```

Before running anything, `wj explain-effects` lists what the program would do, with the call chain to each effect.

## The default is narrow

With no grants at all, a run may use files inside the project directory, plus time, random numbers and console output.
Anything else stops, naming the fix:

```sh,verify
$ cat > snoop.wj <<'EOF'
use std::fs

fn main() {
    let hosts = fs::read_to_string("/etc/hosts").unwrap_or("")
    println("read ${hosts.len() > 0}")
}
EOF
$ wj run snoop.wj 2>&1 || true
capability denied: fs.read(/etc/hosts)
to allow just this once: run with `--allow fs.read=/etc/hosts`
```

The denial is not an error value the program can swallow. It is a policy fault with its own exit status, so a dependency
cannot hide that it tried.

## The grant, in plain sight

Allowing the read is one flag for one run, or one line in `wj.toml` for good:

```sh,verify
$ cat > snoop.wj <<'EOF'
use std::fs

fn main() {
    let hosts = fs::read_to_string("/etc/hosts").unwrap_or("")
    println("read ${hosts.len() > 0}")
}
EOF
$ wj run --allow fs.read=/etc/hosts snoop.wj
read true
```

```toml
[capabilities]
fs.read = ["./**", "/etc/hosts"]
```

The manifest is a file in the repository, so reviewing a change also reviews what the program may do. Hosts for the
network, variable names for the environment and program names for processes are granted the same way, and a dependency has
a grant of its own. `wj caps show` lists what each dependency is known to do.

## Static checks too

`wj check` compares the code with the grant before the program runs and reports a literal resource outside it, with the
call chain that leads there (`WJ0041`). A function declared `@pure` that reaches for a file is reported too (`WJ0042`).
Chapter [Effects and capabilities](../tutorial/effects.md) walks through each of these, and the
[effects model](../reference/effects-model.md) states the rules.
