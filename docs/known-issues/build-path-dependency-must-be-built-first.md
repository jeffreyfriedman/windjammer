# `wj build` / `wj test` of a package fails when a `path = "../pkg/build"` dependency was not built yet

An ecosystem package links its sibling through `wj_glob = { path = "../wj-glob/build" }`. `build/` is generated and git-ignored, so on a clean
checkout `wj build src` in `wj-gitignore` (directory order: before `wj-glob`) wrote `path = "../wj-glob/build"` into its `Cargo.toml` relative to the
wrong directory and cargo reported `failed to read .../wj-gitignore/wj-glob/build/Cargo.toml`. The same shape broke the apps that use
`wj-gitignore` (`wj-find`, `wj-grep`). The CI loop builds `apps/*` before `packages/*`.

Status: fixed. `wj build` (Rust target) and `wj test` build a missing or stale `path = ".../build"` Windjammer dependency first, depth first, so any
directory order works (`cli::path_deps::build_missing`). Gate: `ecosystem_findings2::path_dependencies_build_before_their_dependents`.
