<!-- GENERATED FILE - DO NOT EDIT. Generator: `intrinsics` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# Intrinsics

Built-in functions (ADR-0014 Phase 1), generated from `src/intrinsics/mod.rs`.
The canonical spelling is the plain call; the macro spelling is deprecated (`wj fix`).

| Call | Deprecated macro spelling | Prints | stderr | Newline | Diverges | Message arg |
|---|---|---|---|---|---|---|
| `print` | `print` | yes | - | - | - | 0 |
| `println` | `println` | yes | - | yes | - | 0 |
| `eprint` | `stderr.print` | yes | yes | - | - | 0 |
| `eprintln` | `stderr.println` | yes | yes | yes | - | 0 |
| `format` | `format` | - | - | - | - | 0 |
| `panic` | `panic` | - | - | - | yes | 0 |
| `assert` | `assert` | - | - | - | - | 1 |
| `assert_eq` | `assert_eq` | - | - | - | - | 2 |
| `assert_ne` | `assert_ne` | - | - | - | - | 2 |
| `todo` | `todo` | - | - | - | yes | 0 |
| `unreachable` | `unreachable` | - | - | - | yes | 0 |
| `debug` | `debug` | - | - | - | - | - |
| `vec` | `vec` | - | - | - | - | - |
