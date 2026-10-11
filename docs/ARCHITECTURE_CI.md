# Architecture fitness in CI

The fitness tests live in `tests/architecture.rs` (helpers: `tests/common/arch_scan.rs`) and run as part of
`cargo test --release --test all`. They need no extra tooling, so the main test job already covers them. A
dedicated fast job gives earlier feedback and a clearer failure message.

## Step to add to `.github/workflows/` (owned by another agent; not edited here)

```yaml
  architecture:
    name: Architecture fitness
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Architecture ratchets, MODULE.md and generated graph freshness
        run: cargo test --release --test all architecture
```

The filter `architecture` matches every test in `tests/architecture.rs` and the scanner unit tests.

## What fails the job

| Failure | Meaning | Fix |
|---|---|---|
| `[forbidden_edges] ... regression` | a module imports a module its layer may not use | move the shared item down a layer |
| `[long_functions]` / `[long_files]` regression | a function > 300 lines or file > 2000 lines grew, or a new one appeared | extract named steps / split the file |
| `[env_var_reads]`, `[thread_local]`, `[static_state]` regression | new hidden state or ambient configuration | pass config / session explicitly |
| `[allow_dead_code]`, `[crate_level_allow_dead_code]`, `[get_signature_calls]` regression | count went up | delete the dead code / resolve identities once |
| `... allow-list ... higher than reality` | an improvement landed; the baseline must shrink | run `scripts/depgraph.sh` and commit |
| `docs/generated/deps.* is stale` | import edges changed | run `scripts/depgraph.sh` and commit |
| `MODULE.md problems` / `missing from docs/MODULE_MAP.toml` | new module without docs or map entry | add `MODULE.md` (<= 60 lines, required headings) and a map entry |

## Local workflow

- Check: `cargo test --release --test all architecture`
- After shrinking a number: `scripts/depgraph.sh` (never raises a baseline; it refuses and prints the offender).
- Baselines: `docs/architecture-allowlist.toml`; contracts: `docs/MODULE_MAP.toml`; graph: `docs/generated/deps.md`, `deps.dot`.
