---
status: living
last_verified: 61d60bde8
---
# Interpreter DX: cost, ergonomics, speed

Measured first, changed second. Each item gets one attempt, and the table below records its effect. The driver is
`scripts/wj/dx_measure.wj`:

```
wj run --interpret scripts/wj/dx_measure.wj -- <absolute wj binary> --runs 7
```

**Output cost.** The driver runs every Windjammer draft of agent pilot 4 (the first and final source of each trial)
as the agent ran it: `wj run --interpret main.wj`, with the task's first sample on stdin. It counts the
bytes on stderr, which are the diagnostics, warnings and panics an agent reads on every run. Tokens are estimated as
bytes / 4.

**Speed.** The driver measures `wj run --interpret` and `wj check` on a one-line project and on the `t011_lru_cache`
solution, warm and cold (`.wj-cache` removed). It reports the median wall and CPU milliseconds; `/usr/bin/time` has
10 ms resolution.

## Baseline (61d60bde8, load 10 to 12)

| measure | value |
|---|---|
| runs that wrote to stderr | 47 of 239 |
| stderr bytes, all runs | 108,153 (about 113 tokens per run on average; the largest single run 19,263 bytes) |
| diagnostic headlines on stderr | 310, of which 116 `W0005` explicit `.clone()` warnings and 77 `{}`-format warnings |
| lints printed twice | every error-level lint: once by the pre-run lint report (`error: ... [W0002]` with "(error in `wj build`)"), once by the check gate (`error[W0002]` with the source line) |
| `wj run --interpret`, one-line program | warm 10 ms wall / under 10 ms CPU; cold 60 / 40 ms |
| `wj run --interpret`, t011 solution | warm 10 / under 10 ms; cold 100 / 80 ms |
| `wj check`, one-line program | warm under 10 ms; cold 60 / 40 ms |
| `wj check`, t011 solution | warm under 10 ms; cold 90 / 60 ms |

Warm runs are dominated by process start. Cold runs pay the check and the analysis that the project cache keeps.

## Changes and deltas

(Filled in per item as it lands.)
