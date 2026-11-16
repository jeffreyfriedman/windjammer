## Release profiles (C4)

Ratios are Windjammer with the profile / Rust twin with cargo's default release flags (`-C opt-level=3`), geometric mean over the std-only kernels; `own` = Windjammer with the profile / Windjammer with `cargo-default`.

| profile | n | run cycles vs twin | size vs twin | compile CPU vs twin | run cycles vs own default |
|---|---:|---:|---:|---:|---:|
| cargo-default | 17 | 1.12 | 1.00 | 1.05 | 1.00 |
| lto-fat | 17 | 1.16 | 0.88 | 14.22 | 1.04 |
| lto-thin | 17 | 1.13 | 0.99 | 19.05 | 1.01 |
| lto-fat-cgu1 | 17 | 1.16 | 0.88 | 14.11 | 1.04 |
| lto-fat-cgu1-overflow | 17 | 1.29 | 0.88 | 14.16 | 1.15 |
| lto-thin-overflow | 17 | 1.23 | 0.99 | 19.18 | 1.10 |
| cargo-default-overflow | 17 | 1.25 | 1.00 | 1.03 | 1.12 |
| lto-fat-cgu1-abort | 17 | 1.15 | 0.88 | 13.85 | 1.02 |

### Overflow checks per kernel (Windjammer, `lto-fat-cgu1` with / without `-C overflow-checks=on`, run cycles)

| kernel | with / without | twin, cargo default: with / without |
|---|---:|---:|
| parity/sieve | 0.98 | 1.19 |
| parity/matmul | 1.04 | 0.98 |
| parity/collatz | 1.35 | 1.42 |
| parity/string_build | 1.13 | 0.99 |
| parity/fib | 1.27 | 1.30 |
| parity/vec_fill | 1.13 | 1.55 |
| parity/insertion_sort | 1.02 | 1.00 |
| parity/nbody | 1.03 | 0.83 |
| parity/byte_hash | 1.00 | 0.98 |
| parity/histogram | 0.99 | 1.02 |
| interp/closures | 0.86 | 0.90 |
| interp/fib30 | 1.22 | 1.26 |
| interp/hashmap | 0.91 | 1.03 |
| interp/loop5m | 1.77 | 1.00 |
| interp/nbody | 1.06 | 0.91 |
| interp/sieve | 0.95 | 0.97 |
| interp/strings | 0.98 | 1.01 |
| interp/structs | 1.34 | 1.22 |
