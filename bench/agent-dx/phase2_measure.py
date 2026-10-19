"""ADR-0018 agent-dx measurement: grade the Phase-2 idiomatic rewrites (solutions/wj-phase2/, written as first
drafts with the new features, never edited after grading) against the reference Windjammer solutions.

    WJ_BIN=/path/to/wj python3 phase2_measure.py [--json results/adr0018_phase2.json]

Tokens are a language-neutral proxy: count of `\\w+` runs and single punctuation characters in the source.
"""
import json
import os
import re
import sys

from agentdx.baseline import grade_solution
from agentdx.paths import SOLUTIONS, solution_path

LANGS = ["wj-interp", "wj"]


def tokens(path):
    return len(re.findall(r"\w+|[^\w\s]", open(path).read()))


def main():
    out_json = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
    tasks = sorted(os.listdir(os.path.join(SOLUTIONS, "wj-phase2")))
    rows = []
    for t in tasks:
        new = os.path.join(SOLUTIONS, "wj-phase2", t, "main.wj")
        old = solution_path("wj", t)
        row = {"task": t, "tokens_ref": tokens(old), "tokens_phase2": tokens(new)}
        for lang in LANGS:
            r = grade_solution(lang, t, src=new)
            row[lang] = {"status": r["status"], "passed": r["passed"], "total": r["total"],
                         "diagnostics": [d["message"][:300] for d in r["build"].get("diagnostics", [])[:3]],
                         "failures": [f["message"][:200] for f in r["failures"][:2]]}
            print(f"{t:24s} {lang:10s} {r['status']:12s} {r['passed']}/{r['total']}", flush=True)
        rows.append(row)
    for row in rows:
        d = row["tokens_phase2"] - row["tokens_ref"]
        print(f"{row['task']:24s} tokens {row['tokens_ref']:5d} -> {row['tokens_phase2']:5d} ({d:+d}, {100 * d / row['tokens_ref']:+.0f}%)")
    if out_json:
        with open(out_json, "w") as f:
            json.dump({"note": "ADR-0018 first drafts with Phase-2 features (solutions/wj-phase2)", "rows": rows}, f, indent=1)


if __name__ == "__main__":
    main()
