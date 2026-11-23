# Modules and visibility

| Rule | Behaviour |
|---|---|
| `mod name { items }` | declares a namespace; reach its items with `name::item` |
| `pub` | an item without `pub` is visible only inside its module and its children |
| `use a::b::c` | brings `c` into scope under its own name |
| `use a::b::{c, d}` | imports several |
| `use a::b::c as e` | renames |
| `super::` | reaches the parent module, including its private items |
| paths | `::` separates modules, types and items: `Type::assoc`, `Enum::Variant`, `module::item` |
| `pub use a::b::c` | re-exports `c` from this module: `this_module::c` names it. A module offers a name declared elsewhere only through a `pub use` |
| `pub mod child` | declares the child module; it does not re-export the child's items |
| files | a file `name.wj` is the module `name`; declare it with `mod name` in the importer |
| std | `use std::module`; `std` is always available |

```wj
// expect: 21 20
mod outer {
    pub fn secret() -> int {
        20
    }

    pub fn reveal() -> int {
        secret() + 1
    }

    pub mod inner {
        pub fn up() -> int {
            super::secret()
        }
    }
}

use outer::reveal as show
use outer::inner::up

fn main() {
    println("${show()} ${up()}")
}
```

A private item used from outside its module is a compile-time error naming the item: ``function `helper` is private``.

## Re-exports

`pub mod http` makes `adapters::http` a module; it does not make `adapters::serve` a name. A module re-exports an item of
one of its children only with an explicit `pub use`, the way Rust does:

```wj
// expect: 1
mod adapters {
    pub mod http {
        pub fn serve() -> int {
            1
        }
    }

    pub use http::serve
}

fn main() {
    println("${adapters::serve()}")
}
```

Naming an item through a module that does not re-export it is `WJ0194`, with the line to add as the fix; `wj fix <path>` adds
it. A `pub use http::*` glob is an explicit re-export too.

<!-- ignore: WJ0194 is reported by the project check (name resolution), which a fence does not run -->
```wj,ignore
mod adapters {
    pub mod http {
        pub fn serve() -> int {
            1
        }
    }
}

fn main() {
    println("${adapters::serve()}")
}
```

## Projects and packages

A project is a directory with a `wj.toml`. The file `main.wj` in its `src` folder is the program, and every other file
is a module. A package other projects can depend on has a `lib.wj` and is listed under `[dependencies]` by path or git
URL; its public items appear under its name. See [wj.toml](wj-toml.md) for the manifest and
[Modules and packages](../tutorial/modules.md) for a worked example.

## Standard library paths

`use std::collections::HashMap`, `use std::fs`, `use std::json` and the rest are listed on the [standard library](std.md)
page, generated from the sources.
