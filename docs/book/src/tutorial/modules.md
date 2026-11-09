# Modules and packages

A module groups related items under a name. A package is a project with its own `wj.toml` that other projects can
depend on. Both keep a growing program from becoming one long file.

## Inline modules

`mod name { ... }` defines a module in place. Items are private to their module until marked `pub`, and `use` brings a
name into scope:

```wj
// expect: 12
// expect: 7
mod geometry {
    pub fn area(w: int, h: int) -> int {
        w * h
    }

    pub fn half_perimeter(w: int, h: int) -> int {
        w + h
    }
}

use geometry::area
use geometry::half_perimeter

fn main() {
    println("${area(3, 4)}")
    println("${half_perimeter(3, 4)}")
}
```

Paths use `::`. A private function is not reachable from outside its module: calling it is an error that names the
function and says it is private.

## One module per file

Larger programs put each module in its own file next to `main.wj`. A file `receipt.wj` is the module `receipt`, and
`mod receipt` in the importing file declares it:

```sh,verify
$ mkdir -p shop/src
$ cat > shop/wj.toml <<'EOF'
[package]
name = "shop"
version = "0.1.0"
EOF
$ cat > shop/src/main.wj <<'EOF'
mod pricing
mod receipt

use pricing::total
use receipt::format_line

fn main() {
    let names = ["tea", "cake"]
    let prices: Vec<int> = [3, 5]
    let mut i = 0
    while i < names.len() {
        println(format_line(names[i], prices[i]))
        i += 1
    }
    println("total: ${total(prices)}")
}
EOF
$ cat > shop/src/receipt.wj <<'EOF'
pub fn format_line(name: string, price: int) -> string {
    "${name}: ${price}"
}
EOF
$ cat > shop/src/pricing.wj <<'EOF'
pub fn total(prices: Vec<int>) -> int {
    let mut sum = 0
    for p in prices {
        sum += p
    }
    with_tax(sum)
}

fn with_tax(amount: int) -> int {
    amount + amount / 10
}
EOF
$ cd shop && wj run
tea: 3
cake: 5
total: 8
```

`with_tax` is private to `pricing`; `main` can only call `total`. `wj run` with no arguments finds the project from the
current directory, loads every module it uses, and runs the `main.wj` in `src`.

A module can also be a directory with a `mod.wj` inside it, which suits modules that have submodules of their own.
Programs that use directory modules run in the interpreter today; the compiled path has a rough edge with them that is
listed in [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md), and file modules like the ones above work everywhere.

## Packages

A package is a project that another project lists under `[dependencies]` in its `wj.toml`. A path dependency points at a
directory:

```toml
[dependencies]
util = { path = "../util" }
```

The dependency's public items are then available under its name, as in `use util::twice`. The
[wj.toml reference](../reference/wj-toml.md) has a complete two-package example that runs, and covers git
dependencies, Rust crates, and how `wj.lock` pins what was resolved.

Each package has its own capability grant. A dependency that wants to read files or open sockets needs permission, and
the permission is visible in your project; [Effects and capabilities](effects.md) shows how.

## Testing a module

Tests for a module live next to it in a file ending `_test.wj`, and they `use` the module like any other code. The
[testing chapter](testing.md) shows the layout.
