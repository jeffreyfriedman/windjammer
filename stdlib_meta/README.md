# stdlib_meta/

Hand-authored signature metadata for the standard library (`vec`, `string`, `hashmap`, ...): which
parameters borrow, which methods mutate the receiver. The compiler embeds this directory in the `wj`
binary (`src/embedded_assets.rs`) and loads it as the baseline signature registry
(`merge_wj_meta_signatures_from_dir_only`).

These are the only tracked `*.wj.meta` files in the repository, and the only `*.wj.meta` files the
compiler reads from outside a cache. They are source, not build output: nothing writes them, and they
carry no analysis fingerprint. `.gitignore` ignores every other `*.wj.meta` (negation:
`!stdlib_meta/*.wj.meta`), and `tests/sidecar_hygiene_test.rs` fails if any other one is tracked.
