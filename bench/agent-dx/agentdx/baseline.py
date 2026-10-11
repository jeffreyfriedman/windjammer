"""Grade the reference solutions: produces the correctness matrix (language x task)."""
import json
import os
import shutil
import sys
import tempfile
import time

from . import grade as G
from . import tasks as T
from .paths import SOLUTION_FILE, solution_path


def grade_solution(lang, task_id, src=None):
    src = src or solution_path(lang, task_id)
    if not os.path.isfile(src):
        return {"lang": lang, "task": task_id, "status": "missing", "passed": 0, "total": 0,
                "categories": [], "failures": [], "build": {"ok": False, "diagnostics": [], "seconds": 0}}
    with tempfile.TemporaryDirectory(prefix="agentdx-ws-") as wd:
        shutil.copy(src, os.path.join(wd, SOLUTION_FILE[lang][1]))
        t0 = time.time()
        r = G.grade(lang, task_id, wd)
        r["seconds"] = round(time.time() - t0, 2)
        return r


def _explain(r, log):
    if r["status"] == "build_error":
        for d in r["build"]["diagnostics"][:3]:
            print("    build:", d["category"], d["message"][:200].replace("\n", " | "), file=log)
            if os.environ.get("AGENTDX_DETAIL") and d.get("detail"):
                print("      " + d["detail"].replace("\n", "\n      "), file=log)
    for f in r["failures"][:1]:
        print("    fail:", f["category"], f["message"][:160].replace("\n", " | "), file=log)
        print("      stdin:", repr(f["stdin"][:120]), "args:", f["args"], file=log)
        print("      want:", repr(f["expected"][:120]), file=log)
        print("      got: ", repr(f["actual"][:120]), file=log)


def run_matrix(langs, task_ids, log=sys.stderr):
    out = []
    for tid in task_ids:
        for lang in langs:
            r = grade_solution(lang, tid)
            print(f"{tid:34s} {lang:10s} {r['status']:12s} {r['passed']}/{r['total']}", file=log, flush=True)
            out.append(r)
            if r["status"] not in ("pass", "missing") and os.environ.get("AGENTDX_VERBOSE", "1") == "1":
                _explain(r, log)
    return out
