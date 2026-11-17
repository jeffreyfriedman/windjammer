# bench/economics: Windjammer against hand-written Rust, per cost dimension

The suite behind [docs/economics/DEFINITION.md](../../docs/economics/DEFINITION.md). 78 program pairs, each a
Windjammer program and an idiomatic Rust twin that prints the same output:

| Suite | Pairs | Windjammer | Rust twin |
|---|---:|---|---|
| `dx` | 60 | `bench/agent-dx/solutions/wj/<task>/main.wj` | `bench/agent-dx/solutions/rust/<task>/main.rs` |
| `parity` | 10 | `benchmarks/rust_parity/wj/<k>.wj` | `benchmarks/rust_parity/rust/<k>.rs` |
| `interp` | 8 | `bench/interp/<k>.wj` | `twins/<k>.rs` (this directory) |

`twins/` holds only the twins that did not exist elsewhere. Each is the program a competent Rust programmer writes
first: std only, iterators where they are natural, `entry()`, `write!`, slices for read-only parameters, no unsafe,
same algorithm and constants as the Windjammer kernel. Each prints exactly `bench/interp/expected/<k>.out`.

## Run

From the repository root, through the build slot (the runner calls rustc and cargo):

```
<slot> env WJ_ECOSYSTEM=<windjammer-ecosystem checkout> \
  wj run --interpret scripts/wj/economics.wj -- --target <empty cargo cache dir> --label <name>
wj run --interpret scripts/wj/economics_report.wj -- bench/economics/results/<name>.json --md bench/economics/results/<name>.md
```

Options: `--only dx/t001_gcd_lcm,interp/fib30`, `--suite dx|parity|interp`, `--runs N` (min of N, default 3),
`--no-shipped` (skip the `cargo build --release` of each generated crate), `--wj BIN`.

An empty `--target` makes the first step a measured cold build of the generated crates' dependencies; a reused one
skips it (and says so in the results).

## Results

`results/<label>.json` (raw) and `results/<label>.md` (tables). The current numbers and their reading are in
[docs/economics/RESULTS.md](../../docs/economics/RESULTS.md).
