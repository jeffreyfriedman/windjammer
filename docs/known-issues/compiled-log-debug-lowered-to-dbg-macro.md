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

Status: fixed. A function a `use` brings into scope (`use std::log::debug`, `use std::log::{debug, info}`, `use std::log::*`) shadows a language intrinsic of the same name (ADR-0014), in the compiled lowering and in the interpreter. Gates: `ecosystem_findings2::glob_imported_log_debug_is_the_module_function_not_the_intrinsic`, `unshadowed_debug_is_still_the_intrinsic`, `ecosystem_findings2_interp::imported_std_function_shadows_the_intrinsic_of_the_same_name`.
