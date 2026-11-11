# What we have measured

<!-- feature: showcase-measured -->

Claims about a language are cheap, so this page keeps to what the repository measured, shows the numbers as they are, and says
what they cannot tell you. Everything below is generated from `bench/agent-dx/results/pilot6/report.json`, the output of the
sixth agent pilot. The pilot ran each task with an agent in three setups: plain Rust compiled with `rustc`, Windjammer where
the agent builds first, and Windjammer where the agent is encouraged to run the interpreter first.

## The pilot, in one table

The numbers are per trial, as a mean with a 95% interval that resamples tasks and then trials. The number of trials is:

<!-- wj:gen name=pilot6_value path=n_trials digits=0 -->
0
<!-- wj:end -->

<!-- wj:gen name=pilot6_table -->
<!-- wj:end -->

## What it shows

**Every setup passed every trial.** The pass rate against the hidden tests is the first row of the table, and it is the same
for all three setups. On this task set the model solves the problems in either language, so the pilot cannot rank the
languages by correctness.

**Cost is comparable, and Windjammer is not cheaper.** Plain Rust cost the least per trial, and the paired differences in cost
include zero:

<!-- wj:gen name=pilot6_paired -->
<!-- wj:end -->

Read the last column as the verdict: where the interval contains zero, the data do not show a difference. Windjammer needed
more turns and more input tokens, since the agent reads a short guide and checks its work with the tools. Task by task, the
setup with the lowest cost per trial:

<!-- wj:gen name=pilot6_by_task -->
<!-- wj:end -->

The pilot's notes explain the task where Windjammer cost the least by what that task asks: with only the standard library,
Rust has to hand-write a JSON parser, and Windjammer converts to and from JSON as part of the language. Elsewhere the costs
are close, and on the task with the largest gap the other way, Windjammer cost more.

**The Rust attempts did not fight the compiler.** The number of trials in which the Rust arm hit a `rustc` error:

<!-- wj:gen name=pilot6_value path=rust_errors.trials_with_errors digits=0 -->
0
<!-- wj:end -->

out of this many Rust trials:

<!-- wj:gen name=pilot6_value path=rust_errors.n digits=0 -->
0
<!-- wj:end -->

So on these tasks Rust's borrow checker cost the model nothing, and Windjammer's advantage is not fewer compiler errors. What
Windjammer takes away is the ownership annotation in the source: see [No annotations, the same
Rust](ownership.md), where the program is shorter and the generated Rust carries the borrows.

## What it does not show

* **Whether either language is better for agents in general.** One model, one prompt, ten tasks, three trials per task. The
  intervals are wide and several include zero.
* **Anything beyond the ceiling.** Every setup passed everything, so the task set is too easy to separate them. Harder tasks
  are needed before a pass-rate comparison means anything.
* **The value of the interpreter loop.** The agents in the interpreter-first setup mostly did not iterate in the interpreter:
  they went straight to the verifying run, so that setup measured a compiled-first workflow with an extra check. The
  pilot's own notes say a rerun needs a loop the agents actually follow.
* **Wall-clock time.** The machine was heavily loaded. The number of Rust trials that ran above the pilot's load threshold
  is below, and wall time and tool-wait time are dominated by contention on that machine, where cost, tokens and turns are
  not.

* **A fair Rust cost.** The Rust arm is standard-library-only, one file, no crates and no `cargo`. Real Rust work would use
  crates and pay for `cargo`'s build time, and both move the Rust numbers.
* **Readability, maintenance or what people prefer.** The tests grade behaviour only.

The Rust trials that ran above the load threshold:

<!-- wj:gen name=pilot6_value path=noisy.R.noisy digits=0 -->
0
<!-- wj:end -->

## What the rest of this section does show

The other pages rest on checks that run in this repository, and none depends on the pilot: the programs on
[Side by side](side-by-side.md) are run, and the line counts are computed from them; the speed of the
[edit-run loop](fast-loop.md) is a recorded measurement; the interpreter is compared with the compiled program on
[a corpus](trust.md); the denial and the grant on [Programs that ask for what they need](authority.md) are executed.

When a claim here needs a number, the number is generated from its source and sits next to the claim. A test fails when a page
says "cheaper" or "faster" without one.
