# Load and cache settings observed

- Host load averages during a multi-agent run: 40 to 76 (1, 5 and 15 minute). Holding new build-starting agents until the 1-minute figure drops was the response.
- Subagent build settings: `CARGO_BUILD_JOBS=2` and a per-worktree `CARGO_TARGET_DIR` under the shared cache.
- Timeouts for long builds: 3000 to 3400 seconds, wrapped in `timeout`.
