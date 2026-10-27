"""Loop-mode arms for the agent-preference pilot.

Arms differ ONLY in (a) the loop-mode paragraph and (b) whether the Windjammer context pack is attached.
  A  interpreted-first : check --json / run --interpret / test --interpret; compiled build is the harness's final grade
  B  compiled-first    : build / run --compiled / test on every iteration
  C  no-pack           : loop text of A, but without the context pack (measures the pack's value)
  D  interpreted+build : loop of A, plus ONE `wj build` as a final check before finishing
  P/G/R  python / go / rust baselines: native toolchain loop, no Windjammer pack (the models know these languages)
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
    "interpreted-plus-build": (
        "Development loop (required): iterate with the fast interpreted tools.\n"
        "- `wj check --json main.wj` for diagnostics\n"
        "- `wj run --interpret main.wj` to run the program (pass program input on stdin)\n"
        "- `wj test --interpret .` if you write `*_test.wj` tests\n"
        "Before you finish, run `wj build main.wj` ONCE as a final check that the program also compiles with the production "
        "compiler (the final grade uses it). Run `wj build` again only if that check failed and you changed the code. "
        "Do not use `wj run --compiled`."),
    "native-python": (
        "Development loop: run the program with `python3 main.py` (pass program input on stdin)."),
    "native-go": (
        "Development loop: build and run with `go run main.go` or `go build -o main main.go && ./main` (pass program input on stdin)."),
    "native-rust": (
        "Development loop: compile with `rustc --edition 2021 -o main main.rs` and run `./main` (pass program input on stdin)."),
}

ARMS = {
    "A": {"name": "interpreted-first", "loop": "interpreted-first", "pack": True, "lang": "wj"},
    "B": {"name": "compiled-first", "loop": "compiled-first", "pack": True, "lang": "wj"},
    "C": {"name": "no-pack", "loop": "interpreted-first", "pack": False, "lang": "wj"},
    "D": {"name": "interpreted+final-build", "loop": "interpreted-plus-build", "pack": True, "lang": "wj"},
    "P": {"name": "python", "loop": "native-python", "pack": False, "lang": "python"},
    "G": {"name": "go", "loop": "native-go", "pack": False, "lang": "go"},
    "R": {"name": "rust", "loop": "native-rust", "pack": False, "lang": "rust"},
}
LANG_NAME = {"wj": "Windjammer", "python": "Python 3 (standard library only)", "go": "Go (standard library only, package main)",
             "rust": "Rust (std only, edition 2021)"}
SRC = {"wj": "main.wj", "python": "main.py", "go": "main.go", "rust": "main.rs"}
EXTRA_BASH = {"wj": ["Bash(wj:*)"], "python": ["Bash(python3:*)"], "go": ["Bash(go:*)", "Bash(./main:*)"],
              "rust": ["Bash(rustc:*)", "Bash(./main:*)"]}
SHIMMED = {"wj": "wj", "python": "python3", "go": "go", "rust": "rustc"}   # command logged by the shim

def head(lang):
    grader = "the compiled `wj build` toolchain" if lang == "wj" else "the native toolchain"
    cmds = "`wj`, `printf`, `echo`, `head` and `tail`" if lang == "wj" else {
        "python": "`python3`, `printf`, `echo`, `head` and `tail`", "go": "`go`, `./main`, `printf`, `echo`, `head` and `tail`",
        "rust": "`rustc`, `./main`, `printf`, `echo`, `head` and `tail`"}[lang]
    return (f"You are solving one programming task in {LANG_NAME[lang]}. Write the complete solution in the file `{SRC[lang]}` "
            "in the current directory. The program reads standard input (and command-line arguments / files in the "
            f"current directory when the task says so) and writes to standard output exactly as specified; it is graded "
            f"on hidden tests with {grader}. Create and edit files with the Write/Edit tools; the only shell "
            f"commands available are {cmds} (pipe input with `printf 'a\\nb\\n' | ...`). "
            "Stop when you believe it is correct for all inputs.\n\n")


def prompt(task_id, arm):
    lang = ARMS[arm]["lang"]
    return head(lang) + LOOP_TEXT[ARMS[arm]["loop"]] + "\n\n" + T.render_prompt(task_id, lang)


def tool_policy(arm="A"):
    """Tools available to the trial agent. dontAsk denies everything not allowed; file tools stay in the cwd."""
    return {"tools": "Read,Edit,Write,Glob,Grep,Bash",
            "allowed": ["Read", "Edit", "Write", "Glob", "Grep"] + EXTRA_BASH[ARMS[arm]["lang"]] +
                       ["Bash(printf:*)", "Bash(echo:*)", "Bash(head:*)", "Bash(tail:*)"],   # printf/echo/head/tail pipe input into the program
            "permission_mode": "dontAsk"}


def claude_argv(arm, task_id, model, max_turns, max_trial_usd, claude_bin="claude"):
    pol = tool_policy(arm)
    argv = [claude_bin, "-p", prompt(task_id, arm), "--output-format", "json", "--max-turns", str(max_turns),
            "--model", model, "--permission-mode", pol["permission_mode"], "--tools", pol["tools"],
            "--allowedTools", ",".join(pol["allowed"]), "--setting-sources", "project", "--strict-mcp-config",
            "--disable-slash-commands", "--no-session-persistence", "--max-budget-usd", str(max_trial_usd)]
    if ARMS[arm]["pack"]:
        argv += ["--append-system-prompt-file", PACK_PATH]
    return argv
