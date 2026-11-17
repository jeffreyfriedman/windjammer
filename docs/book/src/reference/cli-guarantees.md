<!-- GENERATED FILE - DO NOT EDIT. Generator: `cli_promises` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# Command and flag guarantees

Generated from `docs/promises.toml`. *Tested* entries have acceptance cases that run through the installed `wj`; *preview* entries are in place and still gaining cases.

| Command or flag | What it does | Status |
|---|---|---|
| `wj add` | Add a dependency to wj.toml | tested |
| `wj add --features` | Comma-separated list of features | tested |
| `wj add --path` | Local path dependency | tested |
| `wj agent-index` | Generate agent index artifacts for MCP (errors, stdlib, spec JSON) | tested |
| `wj agent-index --output` | Output directory | tested |
| `wj build` | Build a Windjammer project | tested |
| `wj build --check` | Run cargo build after transpilation and show errors | preview |
| `wj build --defer-drop` | Defer drop optimization mode (auto, always, never) | tested |
| `wj build --defer-drop-threshold` | Defer drop threshold in bytes (default: 102400 = 100KB) | tested |
| `wj build --filter-file` | Filter errors by file path | preview |
| `wj build --filter-type` | Filter errors by type (error, warning) | preview |
| `wj build --fix` | Automatically apply fixes for fixable errors | preview |
| `wj build --ir-shadow-validate` | Run IR shadow validation (compare solver results against legacy analyzer). Discrepancies are reported as errors instead of warnings (CI gate) | tested |
| `wj build --json` | Output diagnostics as machine-readable JSON (for agent/IDE consumption) | tested |
| `wj build --library` | Library mode: exclude test main() functions from output | tested |
| `wj build --mem-report` | Print resident memory (current and peak MiB) at each build phase to stderr as `[wj-mem]` lines | tested |
| `wj build --metadata` | External crate metadata for cross-crate type inference (NAME=PATH, repeatable) | tested |
| `wj build --module-file` | Auto-generate mod.rs with pub mod declarations and re-exports | tested |
| `wj build --no-cargo` | Skip cargo build after transpilation (transpile only) | tested |
| `wj build --no-generate-cargo-toml` | Skip Cargo.toml generation (use project-maintained manifest) | tested |
| `wj build --no-lint` | Disable Rust leakage linter warnings (style, .unwrap(), .iter(), etc.) | tested |
| `wj build --output` | Output directory (default: ./build) | tested |
| `wj build --quiet` | Show only error count (suppress detailed output) | preview |
| `wj build --raw-errors` | Show raw Rust errors instead of translated Windjammer errors | preview |
| `wj build --release` | Build in release mode | preview |
| `wj build --source-maps` | Generate source maps | tested |
| `wj build --target` | Target platform (rust, javascript, wasm) | tested |
| `wj build --tree-shake` | Enable tree shaking (dead code elimination) | tested |
| `wj build --verbose` | Show verbose error output (includes all notes and suggestions) | preview |
| `wj build --unchecked-arithmetic` | Let integer arithmetic wrap on overflow in release builds (default: it panics, as in debug builds and the interpreter); same as `[build] overflow_checks = false` in wj.toml | tested |
| `wj cache` | Manage the shared Cargo target cache (outside application repos) | tested |
| `wj cache path` | Print the shared (or --verify) Cargo target directory | tested |
| `wj cache path --verify` | Use the ephemeral verify cache (integration / codegen cargo checks) | tested |
| `wj cache prune` | Reclaim space (old incremental; aggressive when disk is low) | tested |
| `wj cache prune --aggressive` | Also clear debug incremental trees when reclaiming | tested |
| `wj cache prune --force` | Also delete what a running build holds or wrote within the last hour (normally left alone) | tested |
| `wj cache setup` | Write env.sh / env.ps1 for bare `cargo` invocations | tested |
| `wj cache status` | Show cache sizes and free disk | tested |
| `wj check` | Type check without building | tested |
| `wj check --json` | Output diagnostics as machine-readable JSON (for agent/IDE consumption) | tested |
| `wj eject` | Eject to a standalone Cargo project (the `wj build` output plus vendored path dependencies) | tested |
| `wj eject --output` | Output directory (must be absent or empty) | tested |
| `wj eject --target` | Target platform (rust, wasm) | tested |
| `wj eject --library` | Library crate (as `wj build --library`) | tested |
| `wj eject --module-file` | Generate mod.rs re-exports (as `wj build --module-file`) | tested |
| `wj clean` | Clean build artifacts and stale temp files | tested |
| `wj clean --all` | Also clean local target/ directories (deep clean) | tested |
| `wj docs` | Search the offline docs index (stdlib signatures, built-in methods, idioms, verified examples, error codes); with no query, generate error documentation | tested |
| `wj docs --format` | Format (html, markdown, json) | tested |
| `wj docs --json` | Print search results as JSON | tested |
| `wj docs --limit` | Maximum number of search results | tested |
| `wj docs --output` | Output directory for generated docs | tested |
| `wj errors` | Browse the diagnostics of a file or project (summary by code, filterable) | tested |
| `wj errors --code` | Show only this diagnostic code (e.g. WJ0010, W0002) | tested |
| `wj errors --severity` | Show only this severity (error, warning) | tested |
| `wj explain` | Explain an error code | tested |
| `wj fix` | Migrate deprecated surface syntax (`println!`, `vec![]`, `format!`, ...) to Windjammer forms | tested |
| `wj fix --check` | Do not write; exit non-zero if any file would change | tested |
| `wj fix --docs` | Also migrate ```windjammer / ```wj code fences in .md files | tested |
| `wj replay` | Step through a recorded or simulated run deterministically: breakpoints, stepping, going back, and the values of bindings at any step, from a script of commands (ADR-0035) | tested |
| `wj replay --script` | Debugger commands, one per line; `-` or no script reads them from standard input | tested |
| `wj replay --json` | Print the debugger events as JSON Lines, one object per line | tested |
| `wj replay --simulate` | Debug the run of the deterministic simulated world instead of a recorded one | tested |
| `wj replay --seed` | With --simulate: the seed of the world | tested |
| `wj replay --faults` | With --simulate: probability that a file or network call fails, drawn from the seed | tested |
| `wj replay --allow` | Grant a capability as the recording did; nothing is performed either way | tested |
| `wj replay --at-effect` | Start stopped at the statement that answers the K-th event of the journal (K counts from 1; a recorded run only) | tested |
| `wj reduce` | Shrink a failing project to a minimal, anonymized reproduction (delta debugging) | tested |
| `wj reduce --oracle` | Bug oracle: `rustc-error E0308[:regex]`, `panic` (alias `crash`), `miscompile`, or `cmd '<shell>'` (exit 0 = bug present) | tested |
| `wj reduce --output` | Output directory for the minimal project and repro.md | tested |
| `wj reduce --jobs` | Candidates evaluated in parallel | tested |
| `wj reduce --seed` | Seed for the deterministic candidate order | tested |
| `wj reduce --max-steps` | Maximum number of oracle runs | tested |
| `wj reduce --time-budget` | Wall-clock budget for the reduction search, in seconds | tested |
| `wj reduce --candidate-timeout` | Timeout for a single oracle run, in seconds | tested |
| `wj reduce --keep-names` | Accept a partially anonymized result when some names cannot be renamed without losing the bug | tested |
| `wj reduce --wj-bin` | Compiler binary used by the oracles (default: this executable) | tested |
| `wj reduce --verbose` | Print progress counts to stderr (never source text) | tested |
| `wj fmt` | Format Windjammer code | tested |
| `wj fmt --check` | Check formatting without applying changes (non-zero exit if changes are needed) | tested |
| `wj fmt --rust` | Format generated Rust crates with `cargo fmt --all` instead of .wj files | tested |
| `wj lint` | Run Rust leakage linter on .wj files (W0001-W0004) | tested |
| `wj lint --strict` | Fail on warnings (for CI) | tested |
| `wj new` | Create a new Windjammer project | tested |
| `wj new --template` | Project template (cli, web, lib, wasm) | tested |
| `wj remove` | Remove a dependency from wj.toml | tested |
| `wj repl` | Start the Windjammerscript REPL (interactive interpreter): a persistent session whose input continues on the next line until it parses, with `:history` and `!N` to recall inputs; it warns when the compiled build would reject an input and runs it anyway | tested |
| `wj eval` | Evaluate Windjammer and print the result: `wj eval 'strings.to_upper("hi")'`, `wj eval 'let x = 6; x * 7'`. A `;` or newline inside a string or a block does not split the input. The common std modules are imported (strings, math, json, time, fs, env, HashMap, HashSet); runs interpreted | tested |
| `wj run` | Compile and run a Windjammer file | tested |
| `wj run --defer-drop` | Defer drop optimization mode (auto, always, never) | tested |
| `wj run --defer-drop-threshold` | Defer drop threshold in bytes (default: 102400 = 100KB) | tested |
| `wj run --speculate` | Interpreted runs: also start a compiled build of the same content in the background (lowest priority, offline, one at a time, skipped on a busy machine); the next interpreted run reports a failed build (ADR-0033) | tested |
| `wj run --fix` | Apply every machine-applicable fix `wj check` reports (such as `.collect()` on an iterator used as a list, WJ0035) to the sources, then run the result | tested |
| `wj run --verify` | Interpret, then build compiled and, when the program only reads stdin and writes to the console, run the compiled program again and compare its output and exit status (ADR-0033). Reports on stderr. The interpreted output streams as the program prints. Stdin is read once for both runs; when no input arrives within 2 s it is taken as empty. When the two runs agree it prints `wj verify: OK` (with the program's own exit status) and exits 0, whatever that status is; a difference or a failed build exits non-zero | tested |
| `wj run --stdin-file` | Read standard input from this file (a plain way to give input without a shell pipe, quoting or `printf`) | tested |
| `wj run --interpret` | Interpret directly (Windjammerscript mode): no cargo, no rustc, instant start. Accepts a file or a project directory. Same source compiles with `wj build` for release | tested |
| `wj run --target` | Target platform (rust, javascript, wasm) | tested |
| `wj self-install` | Install wj and plugins to ~/.wj/bin/ and ensure PATH | tested |
| `wj shader-compile` | Compile .wjsl shader to WGSL (with type checking) | tested |
| `wj shader-compile --output` | Output path for .wgsl file (default: stdout) | tested |
| `wj stats` | Show error statistics | tested |
| `wj stats --clear` | Clear statistics | tested |
| `wj stats --verbose` | Show detailed statistics | tested |
| `wj test` | Run tests | tested |
| `wj test --allow-empty` | Exit 0 when no tests are found (default: finding none is an error) | preview |
| `wj test --copy-runtime` | Force copying windjammer-runtime into the temp tree (overrides path-dep inference) | preview |
| `wj test --compiled` | Also build and run each doctest through the Rust backend (default: interpreter only) | preview |
| `wj test --doc` | Run only the doctests (```wj examples in `///` doc comments) | tested |
| `wj test --filter` | Run only tests matching this pattern | tested |
| `wj test --interpret` | Run tests (`@test` / `test_*` in `*_test.wj`) in the interpreter: no cargo, no rustc | tested |
| `wj test --json` | Output results as JSON for tooling | preview |
| `wj test --library` | Compile the library under test (default when `src/` has `.wj` files) | preview |
| `wj test --list` | List discovered test files and functions, then exit without compiling | tested |
| `wj test --module-file` | Generate scoped `mod.rs` layout after transpile (same as `wj build --module-file`) | preview |
| `wj test --no-generate-cargo-toml` | Skip auto-generated Cargo.toml for library compile | preview |
| `wj test --no-runtime-copy` | Use a Cargo path dependency for windjammer-runtime (no recursive copy into temp). Also the default when `wj.toml` declares a path dep on `windjammer-runtime` | preview |
| `wj test --nocapture` | Show output from passing tests | preview |
| `wj test --output` | Output directory for library compile (default: temp `{lib}/`) | preview |
| `wj test --parallel` | Run tests in parallel (default: true) | tested |
| `wj test --runtime-path` | Explicit path to windjammer-runtime (with `--no-runtime-copy`) | preview |
| `wj test --use-build-dir` | Use a pre-built outbound tree instead of recompiling the library | preview |
| `wj test --use-project-cargo` | Merge `[dependencies]` from project root `Cargo.toml` into the test library crate | preview |
| `wj update` | Update Windjammer to the latest version | preview |
| `wj update --check` | Check for updates without installing | preview |
| `wj update --force` | Force reinstall even if already up to date | preview |
| `wj validate-wjsl` | Validate WJSL shader files (transpile to WGSL and check for errors) | tested |
| `wj explain-ownership` | Explain inferred ownership, clones and numeric widths for a file (rule IDs and use sites) | tested |
| `wj explain-ownership --fn` | Only this function (`name` or `Type::name`) | tested |
| `wj explain-ownership --json` | Machine-readable JSON output | tested |
| `wj explain-effects` | Explain the effects (files, network, environment, processes, time, randomness, FFI) each function performs | tested |
| `wj explain-effects --fn` | Only this function (`name` or `Type::name`) | tested |
| `wj explain-effects --json` | Machine-readable JSON output | tested |
| `wj optimize` | Show what the post-codegen optimizer changes in a file's emitted Rust, with each pass's soundness argument | tested |
| `wj optimize --explain` | Report each pass that changed the file: edit count, soundness section, default state | tested |
| `wj optimize --pass` | Only this pass (an unknown name is an error that lists the known passes) | tested |
| `wj optimize --list` | List every optimizer pass: name, default state, rewrite and soundness section | tested |
| `wj optimize --json` | Machine-readable JSON output | tested |
| `wj caps` | What the project's dependencies are known to do (per-package capabilities and the lock file) | tested |
| `wj caps --json` | Machine-readable JSON output (show) | tested |
| `wj run --sandbox` | Interpreted runs: ignore the project's grants and start from an empty one (only time, random and console); add authority with --allow (ADR-0021) | tested |
| `wj run --caps` | Interpreted runs without a [capabilities] table: `warn` (default) runs outside-the-profile calls with a warning, `enforce` stops them (exit 126), `off` disables the gate | tested |
| `wj run --allow` | Interpreted runs: grant an effect, e.g. `--allow net.connect=api.example.com` | tested |
| `wj check --fix` |  | tested |
| `wj check --watch` | Re-check whenever a .wj file changes | preview |
| `wj run --allow-network` |  | tested |
| `wj run --deterministic-time` |  | tested |
| `wj run --mock-http` |  | tested |
| `wj run --sched-seed` |  | tested |
| `wj run --simulate` | Interpreted runs in the deterministic simulated world: virtual clock, seeded random numbers and scheduler, in-memory files, no network (ADR-0021) | tested |
| `wj run --seed` | With --simulate: the seed of the world; the run is a function of the program and the seed | tested |
| `wj run --faults` | With --simulate: probability that a file or network call fails, drawn from the seed | tested |
| `wj test --simulate` | Run each interpreted test in the deterministic simulated world (ADR-0021) | tested |
| `wj test --seed` | With --simulate: the seed of the world | tested |
| `wj test --seeds` | With --simulate: try every seed in A..B; a test fails with the first failing seed and the command that reproduces it | tested |
| `wj test --faults` | With --simulate: probability that a file or network call fails, drawn from the seed | tested |
| `wj run --journal` | Interpreted runs keep the step journals of durable fn runs in a directory, so a crashed run resumes where it stopped (ADR-0021) | tested |
| `wj run --crash-after` | With --journal: exit abruptly after the N-th journaled step, to test crash recovery | tested |
| `wj run --record` | Interpreted runs write every effect (file, env, process and network results, stdin, real-clock readings) to a replayable log (ADR-0021) | tested |
| `wj run --replay` | Interpreted runs re-run from an effect log: every effect is answered from the log and none is performed (ADR-0021) | tested |
| `wj self-install --dry-run` |  | tested |
| `wj run --bin` | Binary to run (`[[bin]]` in wj.toml, cmd/<NAME>/main.wj or src/bin/<NAME>.wj) | tested |
| `wj run --compiled` | Transpile to Rust and run with cargo (the release path), skipping the interpreter | preview |
| `wj run --no-jit` | Interpreted runs: do not compile hot integer functions to machine code (ADR-0020; the same as `WJ_JIT=0`); they run in the resolved engine's closures | tested |
| `wj run --jit` | Interpreted runs: `on` (default) or `off`: machine code for hot integer functions (ADR-0020) | tested |
| `wj run --jit-loop-hot` | Interpreted runs (internal, hidden from help; env WJ_JIT_LOOP_HOT): closure-engine iterations before a hot loop enters machine code; 0 makes every loop try at its first iteration (stress tests) | tested |
| `wj run --jit-debug` | Interpreted runs (internal, hidden from help; env WJ_JIT_DEBUG): report on stderr which loops run as machine code and why others do not | tested |
| `wj run --jit-mutant` | Interpreted runs (internal, hidden from help; env WJ_JIT_MUTANT): in debug builds, leave one machine-code check out for the mutation gate; release builds ignore it | tested |
| `wj run --pair-list` | Interpreted runs (internal, hidden from help): run every program listed in FILE under the tree walker and the resolved engine in one process, without the check, and print a JSON line per program and engine (exit status, stdout, stderr); the drift fuzz step's batch mode (ADR-0038 C4) | tested |
| `wj run --engine` | Interpreted runs: `resolved` (default: compiled closures, ADR-0020; functions outside its subset run in the tree walker) or `walker` (the tree walker only, the reference semantics) | tested |
| `wj run --hash-seed` | Interpreted runs: perturb HashMap/HashSet iteration order (0 = key order; N = a reproducible shuffle) to expose programs that depend on Rust's unspecified hash order (ADR-0021 D6) | tested |
| `wj run --error-format` | Interpreted runs: how a failed run reports on stderr: `human` (default: the panic message, the failing source line, the call stack and the bindings of the failing frame) or `json` (one line in the diagnostics schema with `stack` and `locals`). Program stdout and the exit status do not change (ADR-0036) | tested |
| `wj run --no-locals` | Interpreted runs: leave the bindings of the failing frame out of the error report (they are redacted when they look like secrets, and this drops them altogether) | tested |
| `wj run --hot` | Interpret and apply edits to function bodies while the program runs, keeping its state (preview, ADR-0021) | preview |
| `wj run --release` | Like --compiled, with optimizations | preview |
| `wj run --watch` | Re-run on every source change (restarts long-running programs) | preview |
| `wj test --watch` | Re-run the tests on every source change | preview |
| `wj run --no-lint` | Interpreted runs: do not refuse the program for Rust-leakage lints (W####, such as `.unwrap()`); real compile errors still refuse it | tested |
| `wj mcp` | MCP tools as typed Windjammer for code mode (ADR-0021) | tested |
| `wj mcp bind` | Generate a typed Windjammer module for the tools of an MCP server: a function per tool, structs for arguments and results | tested |
| `wj mcp bind --tools` | Read the server's tool list (tools/list JSON) from a file, or standard input with - | tested |
| `wj mcp bind --stdio` | Ask a live MCP server for its tool list by starting it as a child process | tested |
| `wj mcp bind --tool` | Bind only the named tool (repeatable), so the program's effect set is as small as the script | tested |
| `wj mcp bind --out` | Write the generated module to a file instead of standard output | tested |
| `wj run --bind` | Let mcp::call reach an MCP server: NAME=ENDPOINT with mock:FILE or stdio:COMMAND (ADR-0021) | tested |
| `wj run --limits` | Resource limits of a run: fuel, wall, output, calls, mem; exceeding one stops it with exit status 124 | tested |
| `wj run --result-json` | Write the exit status, effects used and resources used of a run to a JSON file | tested |
| `wj cache warm` | Build a tiny program once so the shared runtime cache is warm | tested |
| `wj doctor` | Find every wj on PATH, flag stale or conflicting builds and a missing shared cache, and print the fixes (exit 1 on problems) | tested |
| `wj doctor --expect-head` | Only check that the wj PATH resolves first was built from HEAD of the enclosing compiler checkout (merge gate) | tested |
| `wj doctor --strict` | Also fail (exit 1) on warnings, such as a wj built from an older commit than the enclosing checkout | tested |
| `wj doctor --pin` | Print the toolchain pin line for this wj (its source key), ready to paste under `[toolchain]` in wj.toml | tested |
| `wj doctor --json` | Machine-readable output | tested |
| `wj install-dev` | Atomically install this wj (or --from another build) to ~/.cargo/bin/wj, keeping a timestamped backup | tested |
| `wj install-dev --dry-run` | Print what would happen without writing anything | tested |
| `wj install-dev --force` | Install even if it is older than what is installed | tested |
| `wj install-dev --from` | Binary to install (default: the running wj) | tested |
| `wj install-dev --to` | Destination (default: $CARGO_HOME/bin/wj, else ~/.cargo/bin/wj) | tested |
| `wj version` | Show this wj's identity: version, commit, commit date, dirty flag, build profile, build date | tested |
| `wj version --json` | Machine-readable output | tested |
| `wj clean --older-than-hours` | With --tmp: only directories older than this many hours | tested |
| `wj clean --sidecars` | Only remove legacy `*.wj.meta` sidecars next to sources under the current directory | tested |
| `wj clean --tmp` | Remove stale `wj-*` scratch directories from the OS temp dir (dry run unless --yes) | tested |
| `wj clean --yes` | Actually remove what --tmp lists | tested |
| `wj rust-surface` | Generate the signature surface of a Rust crate dependency (ADR-0026) from its source | tested |
| `wj rust-surface --all-pub` | Every public free function that maps (the default) | tested |
| `wj rust-surface --fn` | Only these functions (repeatable); the cache copy always has every function | tested |
| `wj rust-surface --path` | Project directory (default: the current directory) | tested |
| `wj rust-surface --write` | Write an editable copy to rust/<crate>.wji (source: commit it, it wins over the generated one) | tested |
| `wj check --error-format` | Diagnostic format: `human` (default) or `agent`, one compact line per diagnostic (location, code, message, first help, fixes) for tools and coding agents; `WJ_ERROR_FORMAT=agent` sets it for every command | tested |
| `wj run --error-format` | Diagnostic format: `human` (default) or `agent`, one compact line per diagnostic (location, code, message, first help, fixes) for tools and coding agents; `WJ_ERROR_FORMAT=agent` sets it for every command | tested |
| `wj lock` | Resolve dependencies and write or verify wj.lock (exact revisions and content hashes) | tested |
| `wj lock update` | Re-resolve every path and git dependency, in dependency order, and write wj.lock | tested |
| `wj lock check` | Verify that wj.lock matches the manifests and dependency contents; exit 1 when it does not | tested |
| `wj build --locked` | Fail when wj.lock is missing or no longer matches the dependencies (CI and release builds) | tested |
