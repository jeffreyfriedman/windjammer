# `wj build` output depends on the checkout's `.wj-cache/`

`wj build FILE` loads peer signatures from `<project root>/.wj-cache/` (project root = nearest directory above the
source with a `Cargo.toml` or `wj.toml`) and from the source directory. Every earlier build of any file in the same
project writes there. Inside the compiler checkout that is `<checkout>/.wj-cache/`, shared by all of
`examples/`, `tests/conformance_programs/` and `tests/`, so the output for a program can differ between a cold and a
warm cache (`pipe_basics`, `170_mut_ref_value_types` and `173_type_aliases` did: the program's own metadata from a
previous build changed the next build) and between concurrent test runs.

What was done: `tests/codegen_determinism_test.rs` now builds each program from a private copy under its own
`wj.toml`, so the gate measures the compiler and not the cache state.

What remains (compiler): a build must not consume the metadata of the file it is compiling, nor of files it does not
import. Peer metadata should be selected by the module graph (`use` / `mod` edges), not by directory.
