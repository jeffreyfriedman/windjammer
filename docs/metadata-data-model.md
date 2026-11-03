# Metadata data model: `.wj.meta`, `metadata.json`, and where they live

Analysis metadata is the compiler's record of what it learned about a module: function signatures with
the converged ownership of each parameter, struct field types, trait impls, Copy structs. Later
compilations read it to infer across file and crate boundaries.

## Artifacts

| Artifact | Written by | Read by | Location |
|---|---|---|---|
| `<module>.wj.meta` (JSON `ModuleMetadata`, with an `analysis_fingerprint`: content hash + compiler build identity + dependency epoch) | `wj build`, library multipass, `emit_module_meta_for_file*` | peer modules of the same tree (cross-file ownership), the numeric inference (`load_imported_metadata`), the incremental fingerprint check, `wj check` cache keys | the analysis cache, below |
| `metadata.json` (`CrateMetadata`) | `wj build --library` into the output directory | applications that depend on the crate (path dependencies) | output directory (an intentional, published artifact) |
| `stdlib_meta/*.wj.meta` | hand-authored | the compiler, embedded in the binary | tracked in the compiler repo (see `stdlib_meta/README.md`) |

## Where sidecars live

Never next to a source file.

- Project with `wj.toml` or `Cargo.toml`: `<project>/.wj-cache/<path under src>.wj.meta` (git-ignored).
- No manifest above the source: `<temp dir>/wj-meta/<hash of the module root>/<path>.wj.meta` (the OS temp
  dir, honouring `TMPDIR`; it is a cache, so losing it costs a cold build, never correctness), where the module root is found by
  climbing out of module directories (a directory with `mod.wj`, or with a sibling `<name>.wj`), stopping
  at the nearest ancestor named `src`, else the file's directory.

A `*.wj.meta` found anywhere else is a legacy sidecar written by an older compiler. It is never read
(`merge_single_wj_meta_file` accepts only cache locations and the embedded `stdlib_meta/`). `wj doctor`
reports how many exist under the current directory and `wj clean --sidecars` deletes them.

## Determinism

A file's own metadata is an output of that file and never an input to its own build
(`with_own_meta_excluded`); a cold build (no cache) and a warm build emit byte-identical code, enforced by
`tests/cold_warm_determinism_test.rs`. A cached sidecar is also rejected when its fingerprint names a
different compiler build or different source text. `wj eject` is byte-identical to `wj build`
(`tests/eject_equivalence_test.rs`); metadata plays no part in the ejected tree.

## Cross-package metadata

Dependencies publish `metadata.json` in their output directory (and read their own `.wj-cache`), so a
dependent never needs sidecars beside the dependency's sources.

## Gates

`tests/sidecar_hygiene_test.rs`: no tracked `*.wj.meta` outside `stdlib_meta/`; `.gitignore` coverage;
`wj build/check/run/test` leave no new file next to any source (with and without a manifest); legacy
sidecars are ignored; `wj clean --sidecars`; `wj doctor` note. Each detector is mutation-checked.
