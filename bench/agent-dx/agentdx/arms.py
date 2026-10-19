"""Loop-mode arms for the agent-preference pilot.

Arms differ ONLY in (a) the loop-mode paragraph and (b) whether the Windjammer context pack is attached.
  A  interpreted-first : check --json / run --interpret / test --interpret; compiled build is the harness's final grade
  B  compiled-first    : build / run --compiled / test on every iteration
  C  no-pack           : loop text of A, but without the context pack (measures the pack's value)
"""
import os

from . import tasks as T
from .paths import ROOT

PACK_PATH = os.path.normpath(os.path.join(ROOT, "..", "..", "docs", "agent", "WINDJAMMER_FOR_AGENTS.md"))

LOOP_TEXT = {
    "interpreted-first": (
        "Development loop (required): iterate with the fast interpreted tools only.\n"
        "- `wj check --json main.wj` for diagnostics\n"
        "- `wj run --interpret main.wj` to run the program (pass program input on stdin)\n"
        "- `wj test --interpret .` if you write `*_test.wj` tests\n"
        "Do not run `wj build` or `wj run --compiled`: the final grade is produced by compiling your `main.wj` "
        "with the production compiler after you finish, so write code that is valid for both paths."),
    "compiled-first": (
        "Development loop (required): iterate with the compiled production path.\n"
        "- `wj build main.wj` to compile (this is what the final grade uses)\n"
        "- `wj run --compiled main.wj` to build and run the program (pass program input on stdin)\n"
        "- `wj test .` if you write `*_test.wj` tests\n"
        "Do not use `--interpret`."),
}

ARMS = {
    "A": {"name": "interpreted-first", "loop": "interpreted-first", "pack": True},
    "B": {"name": "compiled-first", "loop": "compiled-first", "pack": True},
    "C": {"name": "no-pack", "loop": "interpreted-first", "pack": False},
}

HEAD = ("You are solving one programming task in Windjammer. Write the complete solution in the file `main.wj` "
        "in the current directory. The program reads standard input (and command-line arguments / files in the "
        "current directory when the task says so) and writes to standard output exactly as specified; it is graded "
        "on hidden tests with the compiled `wj build` toolchain. Create and edit files with the Write/Edit tools; the only shell "
        "commands available are `wj`, `printf`, `echo`, `head` and `tail` (pipe input with `printf 'a\\nb\\n' | wj run ...`). "
        "Stop when you believe it is correct for all inputs.\n\n")


def prompt(task_id, arm):
    return HEAD + LOOP_TEXT[ARMS[arm]["loop"]] + "\n\n" + T.render_prompt(task_id, "wj")


def tool_policy():
    """Tools available to the trial agent. dontAsk denies everything not allowed; file tools stay in the cwd."""
    return {"tools": "Read,Edit,Write,Glob,Grep,Bash",
            "allowed": ["Read", "Edit", "Write", "Glob", "Grep", "Bash(wj:*)", "Bash(printf:*)", "Bash(echo:*)",
                        "Bash(head:*)", "Bash(tail:*)"],   # printf/echo/head/tail let the agent pipe input into `wj run`
            "permission_mode": "dontAsk"}


def claude_argv(arm, task_id, model, max_turns, max_trial_usd, claude_bin="claude"):
    pol = tool_policy()
    argv = [claude_bin, "-p", prompt(task_id, arm), "--output-format", "json", "--max-turns", str(max_turns),
            "--model", model, "--permission-mode", pol["permission_mode"], "--tools", pol["tools"],
            "--allowedTools", ",".join(pol["allowed"]), "--setting-sources", "project", "--strict-mcp-config",
            "--disable-slash-commands", "--no-session-persistence", "--max-budget-usd", str(max_trial_usd)]
    if ARMS[arm]["pack"]:
        argv += ["--append-system-prompt-file", PACK_PATH]
    return argv
