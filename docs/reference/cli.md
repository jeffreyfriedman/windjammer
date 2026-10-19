<!-- GENERATED FILE - DO NOT EDIT. Generator: `cli` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# `wj` command-line reference

Generated from the clap definitions in `src/bin/wj_cli_args.rs`.

## Commands

| Command | What it does |
|---|---|
| `wj new` | Create a new Windjammer project |
| `wj build` | Build a Windjammer project |
| `wj run` | Run a Windjammer program: interpreted by default (instant start), compiled when it needs to be |
| `wj repl` | Start the Windjammerscript REPL (interactive interpreter) |
| `wj test` | Run tests |
| `wj fmt` | Format Windjammer code |
| `wj fix` | Migrate deprecated surface syntax (`println!`, `vec![]`, `format!`, ...) to Windjammer forms |
| `wj lint` | Run Rust leakage linter on .wj files (W0001-W0004) |
| `wj check` | Type check without building |
| `wj add` | Add a dependency to wj.toml |
| `wj remove` | Remove a dependency from wj.toml |
| `wj update` | Update Windjammer to the latest version |
| `wj stats` | Show error statistics |
| `wj errors` | Browse the diagnostics of a file or project (summary by code, filterable) |
| `wj docs` | Search the offline docs index (stdlib signatures, built-in methods, idioms, verified examples, error codes); with no query, generate error documentation |
| `wj explain-ownership` | Explain inferred ownership, clones and numeric widths for a file (rule IDs and use sites) |
| `wj explain` | Explain an error code |
| `wj agent-index` | Generate agent index artifacts for MCP (errors, stdlib, spec JSON) |
| `wj shader-compile` | Compile .wjsl shader to WGSL (with type checking) |
| `wj validate-wjsl` | Validate WJSL shader files (transpile to WGSL and check for errors) |
| `wj clean` | Clean build artifacts and stale temp files |
| `wj cache` | Manage the shared Cargo target cache (outside application repos) |
| `wj cache path` | Print the shared (or --verify) Cargo target directory |
| `wj cache status` | Show cache sizes and free disk |
| `wj cache prune` | Reclaim space (old incremental; aggressive when disk is low) |
| `wj cache setup` | Write env.sh / env.ps1 for bare `cargo` invocations |
| `wj self-install` | Install wj and plugins to ~/.wj/bin/ and ensure PATH |

## `wj new`

Create a new Windjammer project

```text
Usage: wj new [OPTIONS] <NAME>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<NAME>` | NAME |  | Project name |
| `-t`, `--template` | TEMPLATE | cli | Project template (cli, web, lib, wasm) |

## `wj build`

Build a Windjammer project

```text
Usage: wj build [OPTIONS] <FILE>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | FILE |  | Path to .wj file or directory |
| `-o`, `--output` | DIR |  | Output directory (default: ./build) |
| `-r`, `--release` | RELEASE |  | Build in release mode |
| `-t`, `--target` | TARGET | rust | Target platform (rust, javascript, wasm) |
| `--defer-drop` | MODE | auto | Defer drop optimization mode (auto, always, never) |
| `--defer-drop-threshold` | BYTES |  | Defer drop threshold in bytes (default: 102400 = 100KB) |
| `--tree-shake` | TREE_SHAKE |  | Enable tree shaking (dead code elimination) |
| `--source-maps` | SOURCE_MAPS |  | Generate source maps |
| `--check` | CHECK |  | Run cargo build after transpilation and show errors |
| `--raw-errors` | RAW_ERRORS |  | Show raw Rust errors instead of translated Windjammer errors |
| `--fix` | FIX |  | Automatically apply fixes for fixable errors |
| `-v`, `--verbose` | VERBOSE |  | Show verbose error output (includes all notes and suggestions) |
| `-q`, `--quiet` | QUIET |  | Show only error count (suppress detailed output) |
| `--filter-file` | PATH |  | Filter errors by file path |
| `--filter-type` | TYPE |  | Filter errors by type (error, warning) |
| `--library` | LIBRARY |  | Library mode: exclude test main() functions from output |
| `--module-file` | MODULE_FILE |  | Auto-generate mod.rs with pub mod declarations and re-exports |
| `--no-cargo` | NO_CARGO |  | Skip cargo build after transpilation (transpile only) |
| `--no-lint` | NO_LINT |  | Disable Rust leakage linter warnings (style, .unwrap(), .iter(), etc.) |
| `--no-generate-cargo-toml` | NO_GENERATE_CARGO_TOML |  | Skip Cargo.toml generation (use project-maintained manifest) |
| `--metadata` | NAME=PATH |  | External crate metadata for cross-crate type inference (NAME=PATH, repeatable) |
| `--json` | JSON |  | Output diagnostics as machine-readable JSON (for agent/IDE consumption) |
| `--ir-shadow-validate` | IR_SHADOW_VALIDATE |  | Run IR shadow validation (compare solver results against legacy analyzer). Discrepancies are reported as errors instead of warnings (CI gate) |

## `wj run`

Run a Windjammer program: interpreted by default (instant start), compiled when it needs to be

```text
Usage: wj run [OPTIONS] [PATH] [ARGS]...
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | PATH |  | A .wj file or a directory; omitted: the project containing the current directory (wj.toml) |
| `<ARGS>` | ARGS |  | Arguments to pass to the program |
| `--bin` | NAME |  | Binary to run (`[[bin]]` in wj.toml, cmd/<NAME>/main.wj or src/bin/<NAME>.wj) |
| `-t`, `--target` | TARGET | rust | Target platform (rust, javascript, wasm); non-rust targets always compile |
| `--interpret` | INTERPRET |  | Interpreter only: never fall back to the compiled path (exit 2 on an unsupported feature) |
| `--compiled` | COMPILED |  | Transpile to Rust and run with cargo (the release path), skipping the interpreter |
| `--release` | RELEASE |  | Like --compiled, with optimizations |
| `--watch` | WATCH |  | Re-run on every source change (restarts long-running programs) |
| `--hot` | HOT |  | Interpret and apply edits to function bodies while the program runs, keeping its state (preview, ADR-0021) |
| `--sched-seed` | N | 0 | Interpreted runs: thread schedule seed (0 = FIFO run-to-block; N explores another reproducible interleaving at every synchronisation point) |
| `--hash-seed` | N | 0 | Interpreted runs: perturb HashMap/HashSet iteration order (0 = key order; N = a reproducible shuffle) to expose programs that depend on Rust's unspecified hash order (ADR-0021 D6) |
| `--engine` | ENGINE | walker | Interpreted runs: `walker` (default) or `resolved` (compiled closures, ADR-0020) |
| `--deterministic-time` | DETERMINISTIC_TIME |  | Interpreted runs: no real clock; time starts at a fixed instant and only `sleep` moves it |
| `--allow-network` | ALLOW_NETWORK |  | Interpreted runs: allow plain http:// client requests (default: the http client refuses) |
| `--mock-http` | FILE |  | Interpreted runs: answer http client calls from a JSON file of canned responses |
| `--defer-drop` | MODE | auto | Defer drop optimization mode (auto, always, never) |
| `--defer-drop-threshold` | BYTES |  | Defer drop threshold in bytes (default: 102400 = 100KB) |

## `wj repl`

Start the Windjammerscript REPL (interactive interpreter)

```text
Usage: wj repl
```

## `wj test`

Run tests

```text
Usage: wj test [OPTIONS] [PATH]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | PATH |  | Directory or file containing tests (defaults to current directory) |
| `-f`, `--filter` | FILTER |  | Run only tests matching this pattern |
| `--nocapture` | NOCAPTURE |  | Show output from passing tests |
| `--parallel` | PARALLEL | true | Run tests in parallel (default: true) |
| `--json` | JSON |  | Output results as JSON for tooling |
| `--library` | LIBRARY | true | Compile the library under test (default when `src/` has `.wj` files) |
| `--module-file` | MODULE_FILE |  | Generate scoped `mod.rs` layout after transpile (same as `wj build --module-file`) |
| `-o`, `--output` | DIR |  | Output directory for library compile (default: temp `{lib}/`) |
| `--use-build-dir` | DIR |  | Use a pre-built outbound tree instead of recompiling the library |
| `--use-project-cargo` | USE_PROJECT_CARGO |  | Merge `[dependencies]` from project root `Cargo.toml` into the test library crate |
| `--no-runtime-copy` | NO_RUNTIME_COPY |  | Use a Cargo path dependency for windjammer-runtime (no recursive copy into temp). Also the default when `wj.toml` declares a path dep on `windjammer-runtime` |
| `--copy-runtime` | COPY_RUNTIME |  | Force copying windjammer-runtime into the temp tree (overrides path-dep inference) |
| `--runtime-path` | PATH |  | Explicit path to windjammer-runtime (with `--no-runtime-copy`) |
| `--no-generate-cargo-toml` | NO_GENERATE_CARGO_TOML |  | Skip auto-generated Cargo.toml for library compile |
| `--list` | LIST |  | List discovered test files and functions, then exit without compiling |
| `--allow-empty` | ALLOW_EMPTY |  | Exit 0 when no tests are found (default: finding none is an error) |
| `--interpret` | INTERPRET |  | Run tests (`@test` / `test_*` in `*_test.wj`) in the interpreter: no cargo, no rustc |
| `--watch` | WATCH |  | Re-run the tests on every source change |
| `--doc` | DOC |  | Run only the doctests (```wj examples in `///` doc comments) |
| `--compiled` | COMPILED |  | Also build and run each doctest through the Rust backend (default: interpreter only) |

## `wj fmt`

Format Windjammer code

```text
Usage: wj fmt [OPTIONS] [PATH]...
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--check` | CHECK |  | Check formatting without applying changes (non-zero exit if changes are needed) |
| `--rust` | RUST |  | Format generated Rust crates with `cargo fmt --all` instead of .wj files |
| `<PATHS>` | PATH |  | Files or directories to format (default: current directory); `-` reads stdin |

## `wj fix`

Migrate deprecated surface syntax (`println!`, `vec![]`, `format!`, ...) to Windjammer forms

```text
Usage: wj fix [OPTIONS] <PATH>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | PATH |  | File or directory to rewrite in place |
| `--check` | CHECK |  | Do not write; exit non-zero if any file would change |
| `--docs` | DOCS |  | Also migrate ```windjammer / ```wj code fences in .md files |

## `wj lint`

Run Rust leakage linter on .wj files (W0001-W0004)

```text
Usage: wj lint [OPTIONS] <PATH>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | PATH |  | File or directory to lint |
| `--strict` | STRICT |  | Fail on warnings (for CI) |

## `wj check`

Type check without building

```text
Usage: wj check [OPTIONS] [PATH]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | PATH |  | Path to .wj file or directory (default: current directory) |
| `--json` | JSON |  | Output diagnostics as machine-readable JSON (docs/diagnostics-schema.json) |
| `--watch` | WATCH |  | Re-check whenever a .wj file changes |
| `--fix` | FIX |  | Apply machine-applicable fixes (W0007/W0008 rewrites, `.as_str()` removal, ...) |

## `wj add`

Add a dependency to wj.toml

```text
Usage: wj add [OPTIONS] <PACKAGE>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PACKAGE>` | PACKAGE |  | Package name |
| `-f`, `--features` | FEATURES |  | Comma-separated list of features |
| `-p`, `--path` | PATH |  | Local path dependency |

## `wj remove`

Remove a dependency from wj.toml

```text
Usage: wj remove <PACKAGE>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PACKAGE>` | PACKAGE |  | Package name |

## `wj update`

Update Windjammer to the latest version

```text
Usage: wj update [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--check` | CHECK |  | Check for updates without installing |
| `--force` | FORCE |  | Force reinstall even if already up to date |

## `wj stats`

Show error statistics

```text
Usage: wj stats [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--clear` | CLEAR |  | Clear statistics |
| `-v`, `--verbose` | VERBOSE |  | Show detailed statistics |

## `wj errors`

Browse the diagnostics of a file or project (summary by code, filterable)

```text
Usage: wj errors [OPTIONS] [PATH]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | PATH |  | .wj file or project directory (default: current directory) |
| `--code` | CODE |  | Show only this diagnostic code (e.g. WJ0010, W0002) |
| `--severity` | SEVERITY |  | Show only this severity (error, warning) |

## `wj docs`

Search the offline docs index (stdlib signatures, built-in methods, idioms, verified examples, error codes); with no query, generate error documentation

```text
Usage: wj docs [OPTIONS] [QUERY]...
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<QUERY>` | QUERY |  | Search terms, e.g. `wj docs hashmap entry` (all terms must match) |
| `--limit` | LIMIT | 8 | Maximum number of search results |
| `--json` | JSON |  | Print search results as JSON |
| `-o`, `--output` | OUTPUT | ./docs/errors | Output directory for generated docs |
| `-f`, `--format` | FORMAT | html | Format (html, markdown, json) |

## `wj explain-ownership`

Explain inferred ownership, clones and numeric widths for a file (rule IDs and use sites)

```text
Usage: wj explain-ownership [OPTIONS] <FILE>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<FILE>` | FILE |  | Windjammer source file |
| `--fn` | NAME |  | Only this function (`name` or `Type::name`) |
| `--json` | JSON |  | Machine-readable JSON output |

## `wj explain`

Explain an error code

```text
Usage: wj explain <CODE>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<CODE>` | CODE |  | Error code to explain (e.g., WJ0001, E0425) |

## `wj agent-index`

Generate agent index artifacts for MCP (errors, stdlib, spec JSON)

```text
Usage: wj agent-index [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `-o`, `--output` | OUTPUT | agent_index | Output directory |

## `wj shader-compile`

Compile .wjsl shader to WGSL (with type checking)

```text
Usage: wj shader-compile [OPTIONS] <FILE>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<INPUT>` | FILE |  | Path to .wjsl file |
| `-o`, `--output` | FILE |  | Output path for .wgsl file (default: stdout) |

## `wj validate-wjsl`

Validate WJSL shader files (transpile to WGSL and check for errors)

```text
Usage: wj validate-wjsl <DIR>
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `<PATH>` | DIR |  | Path to directory containing .wjsl files |

## `wj clean`

Clean build artifacts and stale temp files

```text
Usage: wj clean [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--all` | ALL |  | Also clean local target/ directories (deep clean) |

## `wj cache`

Manage the shared Cargo target cache (outside application repos)

```text
Usage: wj cache <COMMAND>
```

## `wj cache path`

Print the shared (or --verify) Cargo target directory

```text
Usage: wj cache path [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--verify` | VERIFY |  | Use the ephemeral verify cache (integration / codegen cargo checks) |

## `wj cache status`

Show cache sizes and free disk

```text
Usage: wj cache status
```

## `wj cache prune`

Reclaim space (old incremental; aggressive when disk is low)

```text
Usage: wj cache prune [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--aggressive` | AGGRESSIVE |  | Also clear debug incremental trees when reclaiming |

## `wj cache setup`

Write env.sh / env.ps1 for bare `cargo` invocations

```text
Usage: wj cache setup
```

## `wj self-install`

Install wj and plugins to ~/.wj/bin/ and ensure PATH

```text
Usage: wj self-install [OPTIONS]
```

| Argument | Value | Default | Description |
|---|---|---|---|
| `--dry-run` | DRY_RUN |  | Print what would be installed (binary, plugins, embedded std/ files) without writing anything |
