<!-- GENERATED FILE - DO NOT EDIT. Generator: `targets` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# Backend support matrix

Generated from `tests/conformance_programs/SUPPORT.toml` and the program corpus in `tests/conformance_programs/`. A listed pair must fail its oracle; the harness ratchets the list down.

| Backend | Corpus programs | Passing | Known unsupported |
|---|---|---|---|
| `rust` | 135 | 110 | 25 |
| `go` | 135 | 130 | 5 |
| `js` | 135 | 130 | 5 |
| `ts` | 135 | 133 | 2 |

## Known-unsupported pairs by cause

| Backend | Cause | Programs |
|---|---|---|
| `go` | format flags `+` and `#` (sign / 0x prefix) are not implemented by the | 1 |
| `go` | mut-ref-to-int-locals | 1 |
| `go` | portable-generic-into | 1 |
| `go` | portable-type-aliases | 1 |
| `go` | pretty Debug `{:#?}` of an aggregate is not implemented by the portabl | 1 |
| `js` | format flags `+` and `#` (sign / 0x prefix) are not implemented by the | 1 |
| `js` | mut-ref-to-int-parameters | 1 |
| `js` | portable-generic-into | 1 |
| `js` | portable-type-aliases | 1 |
| `js` | pretty Debug `{:#?}` of an aggregate is not implemented by the portabl | 1 |
| `rust` | mut-ref-to-int-parameters | 1 |
| `rust` | rust-closure-returned-as-fn-pointer | 1 |
| `rust` | rust-int-literal-width-inference | 14 |
| `rust` | rust-into-bound-param-borrowed | 1 |
| `rust` | rust-std-runtime-crate-needed-by-rustc | 4 |
| `rust` | rust-str-vs-string-in-collect-and-concat | 2 |
| `rust` | rust-tail-none-cast | 2 |
| `ts` | format flags `+` and `#` (sign / 0x prefix) are not implemented by the | 1 |
| `ts` | pretty Debug `{:#?}` of an aggregate is not implemented by the portabl | 1 |
