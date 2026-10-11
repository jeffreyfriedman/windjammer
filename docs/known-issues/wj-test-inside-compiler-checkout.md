# `wj test` run from inside the compiler checkout fails to resolve `windjammer-runtime`

Repro (verified 2026-10-11, 0.50.0): `cd <compiler checkout> && wj test examples/tests` prints
`Using pre-built library at <checkout>/src` and then cargo fails with
`unable to update <checkout>/src/lib.rs ... single file packages cannot be used as dependencies`; 0 tests run.
The same command from any other directory (`cd /tmp && wj test <checkout>/examples/tests`) passes (8 tests).
Cause (suspected, not investigated): library discovery treats `src/lib.rs` of the current directory as the runtime crate.
Workaround: run `wj test` from outside the compiler checkout.
