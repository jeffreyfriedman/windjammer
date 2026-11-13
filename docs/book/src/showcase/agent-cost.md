# What it costs agents

When an agent writes the code, the bill is paid in tokens and turns. Windjammer's goal is to be the cheapest language
for an agent to write correctly, and this page tracks how far along that road it is. Every number below is generated
from `docs/generated/measured.json`; none is typed by hand.

## Where it stands: the last agent pilot

Pilot 6 gave the same ten tasks to the same model in plain Rust and in two Windjammer setups, three trials each, and
graded every program with hidden tests. Every arm passed every trial, so on this task set the question is cost.

Mean cost per trial in US dollars, plain Rust:

<!-- wj:gen name=metric key=cost.pilot6.R.cost -->
0.033
<!-- wj:end -->

Windjammer, compiled build in the loop:

<!-- wj:gen name=metric key=cost.pilot6.U.cost -->
0.039
<!-- wj:end -->

Windjammer, interpreted first with `wj run --verify` at the end:

<!-- wj:gen name=metric key=cost.pilot6.V2.cost -->
0.04
<!-- wj:end -->

Mean agent turns, the same three arms:

<!-- wj:gen name=metric key=cost.pilot6.R.turns -->
3
<!-- wj:end -->

<!-- wj:gen name=metric key=cost.pilot6.U.turns -->
3.8
<!-- wj:end -->

<!-- wj:gen name=metric key=cost.pilot6.V2.turns -->
4.8
<!-- wj:end -->

So today Windjammer is not yet cheaper than plain Rust for an agent on this task set; the differences are within the
pilot's noise, and the point estimates favour Rust. The extra turns have identifiable causes, and each one is now a
measured, gated item of work: programs the compiled build rejected although they looked right, a string escape that
silently printed the wrong text, and build output the agent had to read on every call. The full report is in
`bench/agent-dx/results/pilot6/report.md`.

## Measured without an agent

A pilot costs money and has wide error bars, so the program is steered by proxies that a script computes from the
compiler alone. The friction corpus (`bench/agent-dx/friction/`) holds every failing call agents made in pilots 3 to 6;
`replay.wj` replays them all against the current compiler.

Friction events recorded in the pilots:

<!-- wj:gen name=metric key=cost.friction.events_recorded -->
353
<!-- wj:end -->

Events with a recorded program, replayed on the current compiler:

<!-- wj:gen name=metric key=cost.friction.events_replayable -->
104
<!-- wj:end -->

Of those, still friction today (an open compiler bug, an unhelpful message, or build noise):

<!-- wj:gen name=metric key=cost.friction.events_open -->
9
<!-- wj:end -->

Compiler bugs among them, each tracked with an owner in `bench/agent-dx/friction/open-bugs.md`:

<!-- wj:gen name=metric key=cost.friction.open_compiler_bugs -->
9
<!-- wj:end -->

Tokens an agent reads per diagnostic from `wj check`, in the default format and with `--error-format agent`:

<!-- wj:gen name=metric key=cost.diag.check_tokens_per_error_human -->
278
<!-- wj:end -->

<!-- wj:gen name=metric key=cost.diag.check_tokens_per_error_agent -->
126
<!-- wj:end -->

Tokens per error from a compiled build the compiler should have accepted (the open compiler bugs), counted
separately so that fixing bugs does not move the diagnostic number:

<!-- wj:gen name=metric key=cost.diag.build_tokens_per_error_human -->
116
<!-- wj:end -->

<!-- wj:gen name=metric key=cost.diag.build_tokens_per_error_agent -->
57
<!-- wj:end -->

Share of error codes seen in the corpus that come with a fix a tool can apply without reading prose:

<!-- wj:gen name=metric key=cost.diag.fix_coverage -->
0
<!-- wj:end -->

Lines of build noise per compiled run (the target is zero):

<!-- wj:gen name=metric key=cost.noise.lines_per_build -->
0
<!-- wj:end -->

Tokens to write the same program, Windjammer over Rust, for the reference solutions of the benchmark's tasks and for the
programs the pilot-6 agents finished with:

<!-- wj:gen name=metric key=cost.write.ref_token_ratio_wj_over_rust -->
1.265
<!-- wj:end -->

<!-- wj:gen name=metric key=cost.write.pilot6_token_ratio_wj_over_rust -->
0.877
<!-- wj:end -->

A test (`tests/agent_cost_ratchet_test.rs`) holds these numbers: friction, noise and tokens per error may only go down,
fix coverage only up, and applying a fix may never create a new diagnostic.

## What changes the number

The levers are ranked in ADR-0037 by tokens saved per unit of work. Already in place: compiled runs print only the
program's output and real diagnostics; `--error-format agent` gives one compact line per diagnostic; and `wj check` warns
when `"$${x}"` would print the braces instead of a dollar sign and the value. Next are the open compiler bugs of the
corpus and a single command for the whole loop, then a pilot on harder tasks to see whether the real cost follows the
proxies.
