# Windjammer Standard Library

`std/*.wj` declares the standard library surface; behaviour lives in the Rust runtime (`windjammer_runtime`), which each
`std` module wraps (for example `json` wraps `serde_json`). Users write `use std::json`, never the wrapped crate.

The list of modules and their signatures is generated, not typed here: [docs/reference/stdlib.md](../docs/reference/stdlib.md)
(from a scan of `std/**/*.wj`; regenerate with `scripts/docs_gen.sh`). The per-module `API.md` files that used to live here described
signatures that were not in the code and were removed; the generated reference replaces them.

## Portability tiers (Rust vs Go / JavaScript / TypeScript)

Every `std` module is implemented by `windjammer_runtime` (Rust). The Go and JavaScript/TypeScript backends ship
runtime shims for a **portable subset** only (`std/runtime/go/wj_std.go`, `std/runtime/js/wj_std.mjs`; the signature
table is `src/codegen/portable/stdlib.rs`). Calls into any other module fail at compile time with
`unsupported on this backend` - they never emit placeholder code.

| tier | modules | Go / JS status |
|------|---------|----------------|
| **portable** (same API, shims + conformance programs) | `math` (sqrt, pow, floor/ceil/round/trunc, trig, exp/ln/log, hypot, min/max/clamp, abs, is_nan...), `strings` (case, trim, split/join, contains/replace, len/char_count, repeat, substring, byte_at), `fs` (read_to_string, write, append, exists, remove_file, create_dir_all, is_file, is_dir), `time` (now_millis, now_unix, timestamp, sleep_millis) | WORKS (programs 110-113) |
| **built into the language** (no `use` needed or `use std::collections::...`) | `collections` (`HashMap`, `HashSet`, `Vec`, `Option`, `Result`, iterators, string methods) | WORKS for integer/string/char/bool map keys |
| **partial shim** | `json` (`std/runtime/{go,js}/wj_json.*` exist; not wired through the portable lowering), `subprocess` (shims exist; not wired) | ABSENT from compiled output |
| **Rust-only** | `http`, `net`, `db`, `crypto`, `jwt`, `compress`, `csv`, `yaml`, `regex`, `url`, `uuid`, `mime`, `random`, `log`, `cli`, `config`, `env`, `process`, `sync`, `async`, `testing`, `test`, `storage`, `dialog`, `compute`, `path` | not available; use the Rust target |

Rule of thumb: if a program only needs the language, collections and the four portable modules, it can target Go, JS and
TS; anything else is Rust-only today. New portable functions must be added in all three places (`stdlib.rs`, both shims)
together with a conformance program.
