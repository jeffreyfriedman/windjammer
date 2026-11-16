## Overflow checks: cost and clawback (C4, PASSES.md P10)

Every build `-C opt-level=3`. `checked` = overflow checks on (what Windjammer ships), without / with the bounded_counter pass; `unchecked` = overflow checks off (not shipped; the floor). Recovered = share of the checks' cost the pass removes.

| kernel | pass edits | instr checked, no pass | instr checked, pass | instr unchecked | cost of checks | recovered | cycles checked, no pass / pass |
|---|---:|---:|---:|---:|---:|---:|---:|
| parity/sieve | 0 | 368.7M | 369.4M | 365.1M | 1.01x | -19% | 0.92 |
| parity/matmul | 0 | 283.4M | 283.6M | 283.1M | 1.00x | -63% | 1.00 |
| parity/collatz | 0 | 3761.5M | 3762.2M | 3041.7M | 1.24x | -0% | 1.00 |
| parity/string_build | 0 | 36.1M | 36.4M | 36.3M | 1.00x | - | 1.03 |
| parity/fib | 0 | 1464.3M | 1463.7M | 1108.4M | 1.32x | 0% | 1.00 |
| parity/vec_fill | 0 | 1535.2M | 1535.6M | 1137.0M | 1.35x | -0% | 1.05 |
| parity/insertion_sort | 0 | 706.0M | 705.5M | 707.4M | 1.00x | - | 0.98 |
| parity/nbody | 0 | 75.8M | 75.8M | 75.9M | 1.00x | - | 0.97 |
| parity/byte_hash | 0 | 325.6M | 326.6M | 327.0M | 1.00x | - | 0.99 |
| parity/histogram | 0 | 338.7M | 340.8M | 280.9M | 1.21x | -4% | 0.98 |
| interp/closures | 1 | 12.6M | 12.6M | 11.6M | 1.09x | 3% | 1.17 |
| interp/fib30 | 0 | 39.3M | 39.2M | 33.2M | 1.18x | 1% | 0.95 |
| interp/hashmap | 3 | 54.0M | 53.2M | 52.8M | 1.02x | 72% | 1.01 |
| interp/loop5m | 2 | 73.4M | 73.4M | 31.6M | 2.32x | -0% | 1.00 |
| interp/nbody | 4 | 16.3M | 16.2M | 16.2M | 1.00x | 97% | 0.98 |
| interp/sieve | 2 | 43.0M | 42.9M | 42.9M | 1.00x | 94% | 1.09 |
| interp/strings | 1 | 149.6M | 148.4M | 148.4M | 1.01x | 105% | 1.07 |
| interp/structs | 1 | 18.1M | 17.9M | 15.4M | 1.17x | 6% | 1.08 |
