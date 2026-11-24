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

_DOCS = os.path.normpath(os.path.join(ROOT, "..", "..", "docs", "agent"))
PACK_PATH = os.path.join(_DOCS, "WINDJAMMER_FOR_AGENTS.md")          # the default 2 KB sheet (arm S and its ablations)
FULL_PACK_PATH = os.path.join(_DOCS, "WINDJAMMER_REFERENCE.md")      # the pack that arms A, B, C(no), D used in pilots 1-3

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
        "- `wj run --compiled main.wj` to build and run the program (pass program input on stdin: `printf 'a\\nb\\n' | wj run --compiled main.wj`, "
        "or from a file you wrote with the Write tool: `wj run --compiled --stdin-file in.txt main.wj`)\n"
        "- `wj test .` if you write `*_test.wj` tests\n"
        "Do not use `--interpret`."),
    "compiled-first-agent": (
        "Development loop (required): iterate with the compiled production path.\n"
        "- `wj build main.wj` to compile (this is what the final grade uses); it prints `run: wj run --compiled main.wj`\n"
        "- `wj run --compiled --stdin-file in.txt main.wj` to build and run the program on the input you wrote to `in.txt`\n"
        "- `wj test .` if you write `*_test.wj` tests\n"
        "There is no `./build/main` to run. Do not use `--interpret`."),
    "interpreted-verify": (
        "Development loop (required): iterate with the fast interpreted tools.\n"
        "- `wj check main.wj` for diagnostics\n"
        "- `wj run --interpret main.wj` to run the program (pass program input on stdin)\n"
        "- `wj test --interpret .` if you write `*_test.wj` tests\n"
        "As the last step before you finish, run `wj run --verify main.wj` (same input on stdin): it runs the interpreter, builds the "
        "program with the production compiler (the final grade uses it) and compares the two outputs. Fix and re-run if it reports a problem. "
        "Do not use `wj run --compiled`."),
    "interpreted-plus-build": (
        "Development loop (required): iterate with the fast interpreted tools.\n"
        "- `wj check --json main.wj` for diagnostics\n"
        "- `wj run --interpret main.wj` to run the program (pass program input on stdin)\n"
        "- `wj test --interpret .` if you write `*_test.wj` tests\n"
        "Before you finish, run `wj build main.wj` ONCE as a final check that the program also compiles with the production "
        "compiler (the final grade uses it). Run `wj build` again only if that check failed and you changed the code. "
        "Do not use `wj run --compiled`."),
    "verify-one-command": (
        "Development loop: one command does everything. After writing `main.wj`, run "
        "`printf '<input>' | wj run --verify main.wj` (or `wj run --verify --stdin-file in.txt main.wj` for an input file): it runs the program at once, builds it with the production compiler "
        "(the final grade uses that build), runs the build on the same input and compares. Read what it reports, fix, and run "
        "it again. You are done when it exits 0 and the output is what the task asks for. No other build or run command is needed."),
    # pilot 8: `wj run --verify` exits 0 whenever the interpreted and compiled runs agree, whatever the program's own status
    "verify-ok": (
        "Development loop: one command does everything. After writing `main.wj`, run "
        "`wj run --verify --stdin-file in.txt main.wj` (write the input to `in.txt` first): it runs the program at once, builds it with the production compiler "
        "(the final grade uses that build), runs the build on the same input and compares. Read what it reports, fix, and run "
        "it again. You are done when it prints `wj verify: OK` (it exits 0 then, even if your program exits 1 on purpose: the line "
        "names the program's status) and the output is what the task asks for. No other build or run command is needed."),
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
    # pack / tool experiment: same compiled-first loop as B, tiny pack (packs/), differing only in tool access
    "E": {"name": "tiny-pack+mcp", "loop": "compiled-first", "pack_file": "tiny_E.md", "mcp": True, "lang": "wj", "bash": "core"},
    "F": {"name": "tiny-pack+wj-docs", "loop": "compiled-first", "pack_file": "tiny_F.md", "bash": "core+docs", "lang": "wj"},
    "T": {"name": "tiny-pack-only", "loop": "compiled-first", "pack_file": "tiny_T.md", "bash": "core", "lang": "wj"},
    # tip ablation (agentdx/ablate.py): S = default sheet, Sx = sheet minus one tip group; same loop and tools as F (Sf drops the docs tools)
    "S": {"name": "sheet", "loop": "compiled-first", "pack_file": "ablation/sheet_full.md", "bash": "core+docs", "lang": "wj"},
    "Sa": {"name": "sheet-minus-strings", "loop": "compiled-first", "pack_file": "ablation/sheet_minus_strings.md", "bash": "core+docs", "lang": "wj"},
    "Sb": {"name": "sheet-minus-ownership", "loop": "compiled-first", "pack_file": "ablation/sheet_minus_ownership.md", "bash": "core+docs", "lang": "wj"},
    "Sc": {"name": "sheet-minus-input", "loop": "compiled-first", "pack_file": "ablation/sheet_minus_input.md", "bash": "core+docs", "lang": "wj"},
    "Sd": {"name": "sheet-minus-len", "loop": "compiled-first", "pack_file": "ablation/sheet_minus_len.md", "bash": "core+docs", "lang": "wj"},
    "Se": {"name": "sheet-minus-printing", "loop": "compiled-first", "pack_file": "ablation/sheet_minus_printing.md", "bash": "core+docs", "lang": "wj"},
    "Sf": {"name": "sheet-minus-tools", "loop": "compiled-first", "pack_file": "ablation/sheet_minus_tools.md", "bash": "core", "lang": "wj"},
    # pilot 4: loop mode with one sheet (loop instruction only in the prompt); N = same loop as M with the v2 sheet
    "K": {"name": "compiled-first+sheet", "loop": "compiled-first", "pack_file": "pilot4/sheet_noloop.md", "bash": "core+docs", "lang": "wj"},
    "L": {"name": "interpreted-first+sheet", "loop": "interpreted-first", "pack_file": "pilot4/sheet_noloop.md", "bash": "core+docs", "lang": "wj"},
    "M": {"name": "interpreted+final-build+sheet", "loop": "interpreted-plus-build", "pack_file": "pilot4/sheet_noloop.md", "bash": "core+docs", "lang": "wj"},
    "N": {"name": "interpreted+final-build+v2", "loop": "interpreted-plus-build", "pack_file": "pilot4/sheet_v2.md", "bash": "core+docs", "lang": "wj"},
    # pilot 5: new task set, the adopted sheet minus its loop bullet; loop from the prompt only
    "U": {"name": "compiled-first", "loop": "compiled-first", "pack_file": "pilot5/sheet_noloop.md", "bash": "core+docs", "lang": "wj"},
    "V": {"name": "interpreted-first", "loop": "interpreted-first", "pack_file": "pilot5/sheet_noloop.md", "bash": "core+docs", "lang": "wj"},
    "W": {"name": "interpreted+final-build", "loop": "interpreted-plus-build", "pack_file": "pilot5/sheet_noloop.md", "bash": "core+docs", "lang": "wj"},
    # pilot 6: U (compiled-first) and R (rust) as before; V2 = interpreted-first with width/String predictions and `wj run --verify` as the last step
    "V2": {"name": "interpreted-first+verify", "loop": "interpreted-verify", "pack_file": "pilot6/sheet_v2.md", "bash": "core+docs", "lang": "wj"},
    # pilot 7: V3 = the loop agents actually follow (one command, measured in pilot 6) with compact diagnostics (ADR-0037)
    "V3": {"name": "verify-one-command", "loop": "verify-one-command", "pack_file": "pilot7/sheet_v3.md", "bash": "core+docs", "lang": "wj",
           "env": {"WJ_ERROR_FORMAT": "agent"}},
    # pilot 8 (design: PILOT8_SHEET_AB.md, not run): idiom block (U8), idioms + sharper opening (U8s), verify exits 0 on agreement (V4)
    "U8": {"name": "compiled-first+idioms", "loop": "compiled-first-agent", "pack_file": "pilot8/sheet_u8.md", "bash": "core+docs", "lang": "wj",
           "env": {"WJ_AGENT": "1"}, "agent_mode": True},
    "U8s": {"name": "compiled-first+idioms+sharp-opening", "loop": "compiled-first-agent", "pack_file": "pilot8/sheet_u8s.md", "bash": "core+docs", "lang": "wj",
           "env": {"WJ_AGENT": "1"}, "agent_mode": True},
    "V4": {"name": "verify-ok", "loop": "verify-ok", "pack_file": "pilot8/sheet_v4.md", "bash": "core+docs", "lang": "wj",
           "env": {"WJ_ERROR_FORMAT": "agent", "WJ_AGENT": "1"}, "agent_mode": True},
    "U7": {"name": "compiled-first", "loop": "compiled-first", "pack_file": "pilot7/sheet_u.md", "bash": "core+docs", "lang": "wj"},
    "P": {"name": "python", "loop": "native-python", "pack": False, "lang": "python"},
    "G": {"name": "go", "loop": "native-go", "pack": False, "lang": "go"},
    "R": {"name": "rust", "loop": "native-rust", "pack": False, "lang": "rust"},
}
PACKS_DIR = os.path.join(ROOT, "packs")
MCP_SERVER = "windjammer"
MCP_TOOLS = ["docs_search", "check_code", "explain_error"]     # the only MCP tools exposed (others are disallowed by name)
WJ_CORE = ["build", "run", "test", "check", "fmt", "lint", "fix"]
LANG_NAME = {"wj": "Windjammer", "python": "Python 3 (standard library only)", "go": "Go (standard library only, package main)",
             "rust": "Rust (std only, edition 2021)"}
SRC = {"wj": "main.wj", "python": "main.py", "go": "main.go", "rust": "main.rs"}
EXTRA_BASH = {"wj": ["Bash(wj:*)"], "python": ["Bash(python3:*)"], "go": ["Bash(go:*)", "Bash(./main:*)"],
              "rust": ["Bash(rustc:*)", "Bash(./main:*)"]}
SHIMMED = {"wj": "wj", "python": "python3", "go": "go", "rust": "rustc"}   # command logged by the shim

# One shell-helper allow set for EVERY arm (the toolchain commands differ by language, the helpers do not). Denied for all arms by
# `dontAsk` (nothing outside this list runs): network tools, git, rm/mv/chmod, package managers, interpreters other than the arm's.
# Claude Code decomposes a compound command (`a && b | c; d`) and requires EVERY part to be allowed, so the set must cover each part the
# agents chain in a verification line (measured on 52 denied trials of pilots 7-8: wj/rustc, printf, echo, head, tail, cat, sed, cd, ls, grep, wc,
# diff, the built binary as ./main or ./build/main). Equal for every arm: the toolchain commands (EXTRA_BASH) are the only per-language entries.
# Not allowed for any arm: cargo (dependency fetch = network), timeout/xargs/find/awk/env (run arbitrary commands), curl/wget/ssh, git, rm/mv/cp/chmod/kill,
# package managers, other interpreters.
# `wj build main.wj` leaves the binary at ./build/target/{debug,release}/main (agents found the path from the build output; pilot 11's first
# verification line ran `./build/target/debug/main` and was denied because only ./main and ./build/main were listed).
COMPILED_BINARIES = ["./main", "./build/main", "./build/target/debug/main", "./build/target/release/main"]
SHELL_HELPERS = ["printf", "echo", "head", "tail", "cat", "sed", "cd", "ls", "wc", "sort", "uniq", "tr", "cut", "grep", "diff"] + COMPILED_BINARIES
SAFE_DENY_NOTE = "no network, no git push, no rm/mv outside the trial dir: none of curl/wget/git/rm/mv/chmod/pip/npm is allowed"


# How an arm feeds a program an input FILE (used when the input has a space before `=`, or `$`, `<`, `>`): the only per-language part.
# Measured against Claude Code 2.1.331 with a local fake model (notes/harness-denials3.md): `cat in.txt | <program>` and
# `wj run ... --stdin-file in.txt main.wj` are both accepted; `printf 'x =y' | ...` and `< in.txt` are refused.
FILE_ROUTE = {"wj": "add `--stdin-file in.txt` to your `wj run` command (for example `wj run --compiled --stdin-file in.txt main.wj`)",
              "python": "run `cat in.txt | python3 main.py`", "go": "run `cat in.txt | ./main`", "rust": "run `cat in.txt | ./main`"}
FAIL_NOTE = {"wj": "a failing `wj` call ends its output with `[exit N]`", "python": "a failing `python3` call ends its output with `[exit N]`",
             "go": "a failing `go` call ends its output with `[exit N]`", "rust": "a failing `rustc` call ends its output with `[exit N]`"}
# Zero-extra-turn route (measured): printf expands octal escapes, and the safety check sees only the escape text, so
# `printf 'set A1 \075A2*3\n' | ./main` is accepted where `printf 'set A1 =A2*3\n'` is refused. `=` is \075, `$` is \044, `<` is \074, `>` is \076.
FILE_INPUT_RULE = ("To pipe input that contains a space followed by `=`, or `$`, `<` or `>`, spell that character as a printf octal escape "
                   "(`=` is `\\075`, `$` is `\\044`, `<` is `\\074`, `>` is `\\076`): `printf 'set A1 \\075A2*3\\n' | ...`. "
                   "For a long input write it ONCE with the Write tool to `in.txt` and {route}.")
WJ_SUBCOMMAND_RULE = (" and call `wj` only with one of its subcommands (`build`, `run`, `test`, `check`, `fmt`, `lint`, `fix`, `docs`, `explain`), "
                      "never bare `wj` or `wj --version`")


def head(lang, agent_mode=False):
    grader = "the compiled `wj build` toolchain" if lang == "wj" else "the native toolchain"
    tool = {"wj": "`wj`", "python": "`python3`", "go": "`go`", "rust": "`rustc`"}[lang]
    cmds = tool + ", the compiled binary (`./main`), `printf`, `echo`, `cat`, `sed`, `cd`, `ls`, `wc`, `sort`, `uniq`, `tr`, `cut`, `grep`, `diff`, `head` and `tail`"
    if agent_mode:
        shell = ("(one plain command each, no pipes, redirects, chains, `$?`, `$()` or heredocs; write program input to a file with the Write tool and pass `--stdin-file FILE`). ")
    else:
        shell = ("(pipe input with `printf 'a\\nb\\n' | ...`; you may chain commands with `|`, `&&` and `;`, but EVERY part of a chain must be one of these commands, otherwise the whole line is refused and nothing runs. "
                 "You are already in the project directory: use relative paths, never `cd` elsewhere or name a path outside it (`..`, `/tmp`, `~`)" + (WJ_SUBCOMMAND_RULE if lang == "wj" else "") + ". "
                 "A Bash line must not contain `$`, backticks, `>` or `<`, or a space followed by `=`, even inside quotes, so there are no shell variables such as `$?`, no redirects (`2>&1`, `> f`, `< f`) "
                 f"and no `timeout` or `pkill`: {FAIL_NOTE[lang]}. " + FILE_INPUT_RULE.format(route=FILE_ROUTE[lang]) + ") ")
    return (f"You are solving one programming task in {LANG_NAME[lang]}. Write the complete solution in the file `{SRC[lang]}` "
            "in the current directory. The program reads standard input (and command-line arguments / files in the "
            f"current directory when the task says so) and writes to standard output exactly as specified; it is graded "
            f"on hidden tests with {grader}. Create and edit files with the Write/Edit tools; the only shell "
            f"commands available are {cmds} " + shell +
            "Stop when you believe it is correct for all inputs.\n\n")


# `wj run|check|build|test` in agent mode (WJ_AGENT=1, ADR-0037): one merged, bounded stream that ends with `exit status: N`
AGENT_MODE_TEXT = (" One command, no redirects, no chains: `wj` output already includes stderr, is bounded, and ends with a line "
                   "`exit status: N`, so do not use `2>&1`, pipes, `&&`, `;`, `$?` or `${PIPESTATUS}` (they are refused). Never write a space followed by `=` in a Bash command (refused even inside quotes): write multi-line input with the Write tool and pass it with `--stdin-file`.")


def head_agent(lang):
    """The pilot-8 task header: input through a file written with the Write tool (`--stdin-file`), no shell input pipes."""
    return head(lang, agent_mode=True)


def prompt(task_id, arm):
    lang = ARMS[arm]["lang"]
    return (head_agent(lang) if ARMS[arm].get("agent_mode") else head(lang)) + LOOP_TEXT[ARMS[arm]["loop"]] + AGENT_MODE_TEXT * bool(ARMS[arm].get("agent_mode")) + "\n\n" + T.render_prompt(task_id, lang, bool(ARMS[arm].get("agent_mode")))


def _bash(arm):
    a = ARMS[arm]
    if a.get("bash") in ("core", "core+docs"):
        subs = WJ_CORE + (["docs", "explain"] if a["bash"] == "core+docs" else [])
        return [f"Bash(wj {c}:*)" for c in subs]
    return EXTRA_BASH[a["lang"]]


def _helpers():
    return [f"Bash({c}:*)" for c in SHELL_HELPERS]


def _mcp_allowed(arm):
    return [f"mcp__{MCP_SERVER}__{t}" for t in MCP_TOOLS] if ARMS[arm].get("mcp") else []


def mcp_config(server_bin):
    import json
    return json.dumps({"mcpServers": {MCP_SERVER: {"command": server_bin, "args": ["stdio"]}}})


def pack_path(arm):
    a = ARMS[arm]
    if a.get("pack_file"):
        return os.path.join(PACKS_DIR, a["pack_file"])
    return FULL_PACK_PATH if a.get("pack") else None


def all_mcp_tools():
    import json
    m = os.path.normpath(os.path.join(ROOT, "..", "..", "crates", "windjammer-mcp", "tools", "manifest.json"))
    m = json.load(open(m))
    return [t["name"] for t in (m["tools"] if isinstance(m, dict) else m)]


def tool_policy(arm="A"):
    """Tools available to the trial agent. dontAsk denies everything not allowed; file tools stay in the cwd."""
    return {"tools": "Read,Edit,Write,Glob,Grep,Bash",
            "allowed": list(dict.fromkeys(["Read", "Edit", "Write", "Glob", "Grep"] + _bash(arm) + _mcp_allowed(arm) + _helpers())),   # identical helpers for every arm
            "permission_mode": "dontAsk"}


def claude_argv(arm, task_id, model, max_turns, max_trial_usd, claude_bin="claude", mcp_config_path=None, stream=False):
    pol = tool_policy(arm)
    argv = [claude_bin, "-p", prompt(task_id, arm), "--output-format", "stream-json" if stream else "json"] + (["--verbose"] if stream else []) + [
            "--max-turns", str(max_turns),
            "--model", model, "--permission-mode", pol["permission_mode"], "--tools", pol["tools"],
            "--allowedTools", ",".join(pol["allowed"]), "--setting-sources", "project", "--strict-mcp-config",
            "--disable-slash-commands", "--no-session-persistence", "--max-budget-usd", str(max_trial_usd)]
    pk = pack_path(arm)
    if pk:
        argv += ["--append-system-prompt-file", pk]
    if ARMS[arm].get("mcp"):
        hidden = [f"mcp__{MCP_SERVER}__{t}" for t in all_mcp_tools() if t not in MCP_TOOLS]
        argv += ["--mcp-config", mcp_config_path, "--disallowedTools", ",".join(hidden)]
    return argv
