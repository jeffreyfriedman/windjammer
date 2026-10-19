# `std::process::pid()` is declared in `std/process.wj` but the runtime has no `pid`

```windjammer
use std::process

fn main() {
    println("${process.pid()}")
}
```

`wj build` emits `process::pid()` and rustc fails with `E0425: cannot find function pid in module process`. The declaration
exists (`fn pid() -> int`); the runtime function does not. Either implement it (`std::process::id()`) or remove the
declaration.
