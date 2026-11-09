<!-- GENERATED FILE - DO NOT EDIT. Generator: `targets` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# Backend support matrix

Generated from `tests/conformance_programs/SUPPORT.toml` and the program corpus in `tests/conformance_programs/`. A listed pair must fail its oracle; the harness ratchets the list down.

| Backend | Corpus programs | Passing | Known unsupported |
|---|---|---|---|
| `rust` | 156 | 129 | 27 |
| `go` | 156 | 156 | 0 |
| `js` | 156 | 156 | 0 |

## Known-unsupported pairs by cause

| Backend | Cause | Programs |
|---|---|---|
| `rust` | mut-ref-to-int-parameters | 2 |
| `rust` | rust-closure-field-move | 1 |
| `rust` | rust-closure-if-tail | 1 |
| `rust` | rust-int-literal-width-inference | 11 |
| `rust` | rust-into-bound-param-borrowed | 1 |
| `rust` | rust-module-impl-self-type | 1 |
| `rust` | rust-optional-chain | 1 |
| `rust` | rust-std-runtime-crate-needed-by-rustc | 4 |
| `rust` | rust-str-vs-string-in-collect-and-concat | 2 |
| `rust` | rust-tail-none-cast | 2 |
| `rust` | rust-unsized-slice-binding | 1 |
