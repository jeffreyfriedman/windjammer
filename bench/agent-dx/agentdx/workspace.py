"""Agent workspace: what the agent sees (prompt, starter file, check script) - never the oracle."""
import json
import os
import shutil
import sys
import time

from . import grade as G
from . import tasks as T
from .paths import ROOT, SOLUTION_FILE

STARTERS = {
    "wj": "// Windjammer solution. Read stdin, write stdout. See PROMPT.md.\nfn main() {\n}\n",
    "rust": "fn main() {\n}\n",
    "go": "package main\n\nfunc main() {\n}\n",
    "python": "",
    "ts": "",
}


def src_name(lang):
    return SOLUTION_FILE[lang][1]


def starter(lang):
    return STARTERS[SOLUTION_FILE[lang][0]]


def make(task_id, lang, wd, log_path):
    """Populate wd with PROMPT.md, starter file and a `check` script."""
    os.makedirs(wd, exist_ok=True)
    with open(os.path.join(wd, "PROMPT.md"), "w") as f:
        f.write(prompt_text(task_id, lang))
    with open(os.path.join(wd, src_name(lang)), "w") as f:
        f.write(starter(lang))
    script = (f"#!/bin/sh\n# Builds the solution and runs it on the visible examples.\n"
              f"PYTHONPATH={ROOT} exec python3 -m agentdx check-workspace --workspace \"$(pwd)\" "
              f"--lang {lang} --task {task_id} --log {log_path}\n")
    p = os.path.join(wd, "check")
    with open(p, "w") as f:
        f.write(script)
    os.chmod(p, 0o755)


def prompt_text(task_id, lang):
    names = {"wj": "Windjammer (compiled via `wj build`)", "wj-interp": "Windjammer (run with `wj run --interpret main.wj`)",
             "rust": "Rust (single file, std only, edition 2021)", "go": "Go (single file, package main, std only)",
             "python": "Python 3 (single file, std only)", "ts": "TypeScript (single file, node built-ins only; run by node with types stripped)"}
    head = (f"You are solving one programming task in {names[lang]}.\n"
            f"Write the complete solution in the file `{src_name(lang)}` in the current directory. "
            "The program reads standard input (and command-line arguments / files in the current directory when the task says so) "
            "and writes to standard output exactly as specified; it is graded on hidden tests.\n"
            "Run `./check` to build and try your program on the visible examples. Stop when it passes and you believe it is correct for all inputs.\n\n")
    return head + T.render_prompt(task_id, lang)


def check(task_id, lang, wd, log_path, out=sys.stdout):
    """Build + run visible samples; append an event to the JSONL log; print feedback."""
    r = G.grade(lang, task_id, wd, hidden=False)
    src = ""
    p = os.path.join(wd, src_name(lang))
    if os.path.isfile(p):
        src = open(p).read()
    ev = {"time": time.time(), "source_kind": "agent_check", "source": src, "build_ok": r["build"]["ok"],
          "categories": r["categories"], "samples_passed": r["passed"], "samples_total": r["total"], "status": r["status"]}
    with open(log_path, "a") as f:
        f.write(json.dumps(ev) + "\n")
    out.write(feedback_text(r))
    return 0 if r["status"] == "pass" else 1


def feedback_text(r):
    """Human/agent-readable feedback from a visible-sample grade result (no hidden info)."""
    lines = []
    if r["status"] == "build_error":
        lines.append("BUILD FAILED")
        for d in r["build"]["diagnostics"][:10]:
            lines.append(f"  [{d['category']}] {d['code']} {d['message']}")
        if not r["build"]["diagnostics"]:
            lines.append(r["build"]["raw"][-1500:])
    else:
        lines.append(f"examples passed: {r['passed']}/{r['total']}")
        for f in r["failures"]:
            lines.append(f"FAILED example {f['index'] + 1} ({f['category']}): {f['message']}")
            lines.append(f"  stdin: {f['stdin']!r}  args: {f['args']}")
            lines.append(f"  expected: {f['expected']!r}")
            lines.append(f"  actual:   {f['actual']!r}")
    return "\n".join(lines) + "\n"
