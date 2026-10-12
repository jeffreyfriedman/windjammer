# Arena benchmarks (ADR-0016)

Two runners, both report the machine load average next to the numbers (measure only on a quiet machine):

- `src/main.rs` (Rust, `cargo run --release -- [reps] [results.json]`): three workloads, each in several variants
  that must print the same checksum: per-frame particles (per-object `Box`, fresh `Vec` per frame,
  `windjammer_runtime::arena::Arena<T>`, hand-written `bumpalo`), AST-like tree building (`Box` nodes, `Handle` nodes,
  `bumpalo` references) and per-frame scratch labels (`String` per label, `Scratch` string, `Scratch::fmt`, one
  reused buffer). A counting allocator reports heap allocations per frame.
- `run_wj.py --wj <wj binary>`: compiled Windjammer programs in `wj/` (`particles_vec` vs `particles_arena`;
  `labels_loop` with and without the auto-arena pass, `WJ_AUTO_ARENA=1`). The compiler must be built with
  `--features unstable_arenas`.

`results.json` is the last recorded run of `src/main.rs`; read it, do not retype it.
