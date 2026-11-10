# repro_packages

Neutral compiler repros that need a package (a library plus a test crate, or a `[capabilities]` grant) and so cannot be a
single-file row of the differential corpus. Each directory has a README naming the owner; run one with
`wj test --interpret` and `wj test` and compare.
