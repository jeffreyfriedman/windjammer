---
name: parallel-cargo-build-throttling
description: Use when parallel cargo builds overload the host.
category: build-workflow
---

# Parallel cargo build throttling

Before starting another cargo-building agent or background build, check host load with `uptime`. Hold new starts while the 1-minute load is well above the core count, and start them as load falls.

- Export `CARGO_BUILD_JOBS=2` in every subagent build command, so parallel agents do not each take every core.
- Give each worktree its own `CARGO_TARGET_DIR` under the shared cache. `wj cache path` prints the root (see AGENTS.md, Shared cargo cache).
- Wrap long builds in `timeout` (3000 to 3400 seconds was used) and filter test output with `grep -E "^test |test result|FAILED|panicked|^error" -A3` to keep logs small.

Observed numbers and settings: [references/load-and-cache-settings.md](references/load-and-cache-settings.md).