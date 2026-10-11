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


def run_matrix(langs, task_ids, log=sys.stderr):
    out = []
    for tid in task_ids:
        for lang in langs:
            r = grade_solution(lang, tid)
            print(f"{tid:34s} {lang:10s} {r['status']:12s} {r['passed']}/{r['total']}", file=log, flush=True)
            out.append(r)
    return out
