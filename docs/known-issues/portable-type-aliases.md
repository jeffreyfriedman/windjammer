# Portable backends (Go / JavaScript / TypeScript) do not expand `type` aliases

```windjammer
type Pair<T> = (T, T)
fn first(p: Pair<int>) -> int { p.0 }
```

`wj build --target go|javascript` fails with an explicit `unsupported on this backend: field `0` on
non-struct `Pair`` error: the portable lowerer (`src/codegen/portable/lower`) treats an alias name as
an unknown nominal type instead of substituting its target (with the alias's type arguments).
Generic and non-generic aliases are both affected. The Rust backend and the interpreter handle them.

Ratchet: `tests/conformance_programs/163_trait_generics_and_aliases` is listed in `SUPPORT.toml` for
`go` and `js`; the entries must be removed when alias expansion lands (the harness fails if they pass).
Fix sketch: collect `Item::TypeAlias` into the lowering env and expand in `ty::from_ast`.
