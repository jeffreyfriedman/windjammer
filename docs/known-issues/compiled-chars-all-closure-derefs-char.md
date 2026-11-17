# Compiled path: a closure over `chars()` dereferences its `char` parameter

Found by the economics suite (docs/economics/RESULTS.md, LTO study; 2026-11-16) on the wave-3 head (cf73e7fe4); the
compiler of 2026-11-11 (37d50458ec6a) built the same program.

`bench/agent-dx/solutions/wj/t009_rpn_eval/main.wj`:

```windjammer
let ok = !tok.starts_with("+") && tok.chars().all(|c| c == '-' || c >= '0' && c <= '9')
```

emits `tok.chars().all(|c| *c == '-' || *c >= '0' && *c <= '9')`; `Iterator::all` over `Chars` passes `char` by value,
so rustc rejects it (E0614, three times). The interpreter runs the program correctly. `all`/`any` closures over
iterators of `Copy` items by value need the parameter used as a value (the deref fits only iterators over references,
such as `v.iter()`).

Repro: `wj build --no-cargo` of the file above, then `cargo build` in the output directory.
