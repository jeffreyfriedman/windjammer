<!-- GENERATED FILE - DO NOT EDIT. Generator: `errors` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# Error and lint codes

Generated from `src/error_codes.rs`, `src/error_catalog.rs` and `src/linter/codes.rs`. Long-form text: `wj explain <CODE>`.

## WJ codes

| Code | Title | Maturity | Rust codes | Summary |
|---|---|---|---|---|
| `WJ0001` | Variable not found | Proven | E0425 | The compiler cannot find a variable with this name in the current scope |
| `WJ0002` | Function not found | Proven | E0425 | The compiler cannot find a function with this name |
| `WJ0003` | Type mismatch | Proven | E0308 | The compiler expected one type but found another |
| `WJ0004` | Cannot modify immutable variable | Proven | E0384, E0596 | Variables in Windjammer are immutable by default |
| `WJ0005` | Type not found | Proven | E0412 | The compiler cannot find a type with this name |
| `WJ0006` | Module not found | Proven | E0433 | The compiler cannot find a module with this name |
| `WJ0007` | Value was moved | Proven | E0382 | This value was moved to another location and can no longer be used here |
| `WJ0008` | Cannot borrow as mutable | Proven | E0502, E0503 | You're trying to borrow a value as mutable, but it's already borrowed as immutable, or vice versa |
| `WJ0009` | Missing field in struct | Proven | E0063 | When creating a struct, you must provide values for all fields |
| `WJ0010` | Pattern match not exhaustive | Proven | E0004 | Your match expression doesn't cover all possible cases |
| `WJ0051` | Local binding changed but not declared mut | HighConfidence |  | A local binding (`let`, a `for` loop variable) is immutable unless declared `mut` (ADR-0002) |
| `WJ0160` | Arena value escapes its arena | HighConfidence | E0597, E0505 | A value allocated in an arena (`Arena::alloc`, `Scratch::str`, `new_string`, `new_list`) is used where it would outlive the arena: returned from the function that declares the arena, stored in a variable, field or collection declared outside the arena's scope, used as the value of the block that declares the arena, or used after the arena's `reset()` |
| `WJ0161` | `std::arena` is unstable | Proven |  | Arenas (ADR-0025) are behind the compiler feature `unstable_arenas` |
| `WJ0173` | Untrusted data reaches a command, file path or query | HighConfidence |  | Data the program did not produce (file contents, standard input, an HTTP request, a database row) flows into a command, the path of a file write or SQL text without passing through a sanitizer (ADR-0009) |
| `WJ0190` | Task is never waited on | HighConfidence |  | Every task (`spawn`, `async`, or a call returning `Task<T>`) is waited on: inside a `scope { }`, which waits for it, or through `.wait()` (ADR-0004) |
| `WJ0191` | Removed concurrency syntax | Proven |  | `thread { }`, `ch <- v` and `<-ch` were removed by ADR-0004: write `spawn { }`, `ch.send(v)` and `ch.recv()`; `wj fix` rewrites them |
| `WJ0192` | Task changes a captured variable | HighConfidence |  | A task works on its own copy of the variables it reads; an assignment to one inside the task is refused (ADR-0004) |
| `WJ0193` | Value cannot move to another thread | HighConfidence |  | A `spawn` task captures a value bound to its thread (`Rc`, `RefCell`, `Cell`, `Weak`); use `async` or build the value inside the task (ADR-0004) |
| `WJ0194` | Name is not re-exported | HighConfidence | E0425, E0433 | A path or import names an item through a module that only gets it from one of its child modules; `pub mod child` re-exports nothing, a module re-exports a name only through an explicit `pub use` (ADR-0040) |
| `WJ0195` | Undefined shader function | HighConfidence |  | A WJSL shader calls a name that is not a WGSL builtin, a value constructor, a struct or a function declared with `fn` in the shader; reported at build time instead of failing later in the WGSL validator |

## Error catalog (translated rustc and intrinsic codes)

| Code | Title | Category | Severity |
|---|---|---|---|
| `E0308` | Type mismatch | Type Errors | error |
| `E0384` | Cannot modify immutable variable | Ownership Errors | error |
| `E0425` | Variable not found | Type Errors | error |
| `W0007` | Deprecated macro spelling (`println!`, `vec![]`, ...) | Style | warning |
| `W0008` | `{}`-style format string | Style | warning |

## Lint codes (W)

| Code | Name | Default level | Summary |
|---|---|---|---|
| `W0001` | explicit-ownership | error | explicit ownership annotation (`&`, `&mut`) on a parameter |
| `W0002` | explicit-unwrap | error | explicit `.unwrap()` / `.expect()` call |
| `W0003` | explicit-iter | error | explicit `.iter()` / `.iter_mut()` call |
| `W0004` | explicit-borrow | error | explicit borrow in a function call |
| `W0005` | explicit-clone | warning | explicit `.clone()` call |
| `W0006` | string-literal-to-string | error | string literal `.to_string()` is Rust leakage |
| `W0010` | non-canonical-string-type | error | `String`/`&str` spelled in source; Windjammer has one string type, `string` (emitted by the parser) |
