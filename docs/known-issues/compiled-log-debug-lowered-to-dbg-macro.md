# Compiled: a call to the `std::log` function `debug(line)` is lowered to Rust's `dbg!`

```windjammer
use std::log::*

fn say(level: string, line: string) {
    match level {
        "debug" => debug(line),
        _ => info(line),
    }
}

fn main() {
    say("debug", "hello")
}
```

`wj build` emits `{ let __wj_dbg = line.clone(); eprintln!("[{}:{}:{}] {} = {:#?}", file!(), ...); __wj_dbg }` for the
`debug(line)` arm (`E0308: expected (), found String`). `debug` here is the logging function imported by `use std::log::*`,
not the `dbg` debugging helper; the lowering must key on the resolved definition, not the bare name.
