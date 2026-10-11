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
