---
name: wj-tip-gate
description: Validate a compiler fix against whole-program multipass builds of the dogfood projects before declaring it done.
---
1. `export CARGO_TARGET_DIR="$(wj cache path)"; cargo build --release`.
2. Run the isolate test, then the multipass/tip test for the same behavior.
3. Rebuild each dogfood project with the new `wj` (ecosystem packages, DownstreamApp, db-project crates, game core) from a scratch copy; compare generated Rust and compile with rustc.
4. Run `cargo test --release --test all`; record before/after counts.
5. Done only if all pass; otherwise report which project/file regressed.
