---
status: current
last_verified: 7ffcc9708733
---
# Soundness: literal widths shared between the tests of one `wj test --interpret` run

Finding: `wj test --interpret` starts every test in a fresh `Interpreter` (`run_one_test` in
`src/interpreter/runner.rs`) that loads the test's whole project again (`load_test_program`: each source file of the
project plus the test file is parsed anew). The first unsuffixed integer literal a module evaluates asks for the module's
literal widths (`Interpreter::compute_literal_widths` in `src/interpreter/compiled_facts.rs`), which runs the compiler's
analysis (`Analyzer::analyze_program`, the IR lowering) on that file. Nothing carried the answer from one test to the
next, so a package with N tests analysed each of its files about N times. The `wj run` path already stores the answer by
content (`share_run_facts`, `src/cli/check.rs`; the store is `src/check/run_cache.rs`); `wj test --interpret` neither
installed that store nor shared in memory.

What the analysis reads. `solve_literal_widths` takes the parsed program of ONE file and nothing else from the project:
`Analyzer::new()` is fresh, `local_struct_field_table(program)` reads the same program, and the stdlib tables
(`StdlibTables::load_or_compute`) and the analyzer's registry layer are fixed by the compiler build (the layer store is
keyed by it). It writes nothing but the returned map from source position (line, column) to an integer width. A parse is a
function of the file's text, its name and the identifier renames applied (`parse_source_renaming` in
`src/interpreter/loader.rs`); `note_parsed` hashes exactly those three (plus a format tag) into the file's content key.

The key. Inside one process, `WidthShare` is keyed by that content key. The compiler build, the baked stdlib and the
`WJ_*` environment are constants of the process, so they cannot differ between two entries of one share, and the share
lives for one run of the test runner only (it is created in `run_specs_with`, passed to each interpreter as a field, never
a static). Across processes the existing store answers: its key adds the compiler build by content, the baked stdlib, the
working directory, the `WJ_*` environment and the `.wj.meta` sidecars a file's `use`s can reach (documented at
`FactStore` in `src/interpreter/compiled_facts.rs`), and it never answers when any of them changed.

Why a hit equals a fresh computation. Equal key means equal text, name and renames, hence an identical parse; the
analysis is a function of that parse and of process constants, so it returns the same map. The answer includes the failure
case (`None`: the analysis failed or the file is not width sensitive), so a hit never turns a refusal into an answer. A
file edited between two runs has a different text, hence a different key: it is analysed afresh. Two tests of one file see
the same text, so they share. A verifying share (`WidthShare::verifying`) recomputes every hit and counts a differing
answer; the gate requires zero.

Residual assumption (the same one the persistent store makes): a `.wj.meta` sidecar or a registry changed by a test
while the run is in progress is not seen by the share. Tests do not write these.

Counters: `WJ_JIT_DEBUG` prints `[fast] widthshare hits H misses M` on stderr after a test run; a miss is one analysis.
With N tests over F files that evaluate literals, misses is F and hits is (N-1)*F.

Pinned by `tests/interpreter_widthshare_test.rs`: a shared answer equals a fresh one over the ecosystem packages' test
files and the repo's engine-parity programs; a changed test file recomputes only its own text; two tests of one module
analyse once; parity of outputs with and without the share. Mutations of the key (text ignored, share ignoring the
file) fail those tests.
