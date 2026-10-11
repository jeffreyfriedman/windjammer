"""Markdown + JSON report generator over baseline results and agent transcripts."""
import glob
import json
import os
from collections import Counter, defaultdict

from . import metrics, tasks as T
from .paths import LANGS

SYMBOL = {"pass": "P", "fail": "F", "build_error": "B", "missing": "-", "skipped": "s"}


def load_transcripts(directory):
    out = []
    for p in sorted(glob.glob(os.path.join(directory, "**", "*.json"), recursive=True)):
        try:
            d = json.load(open(p))
        except ValueError:
            continue
        if isinstance(d, dict) and d.get("schema") == 1 and "iterations" in d:
            out.append(d)
    return out


def matrix_section(results, langs, task_ids):
    by = {(r["task"], r["lang"]): r for r in results}
    tiers = {t: T.load_task(t)["tier"] for t in task_ids}
    lines = ["## Correctness matrix (reference solutions vs hidden vectors)", "",
             "P = all hidden vectors pass, F = runs but wrong, B = does not build/run (compile or interpreter error), - = no solution, s = skipped",
             "", "| task | tier | " + " | ".join(langs) + " |", "|---|---|" + "---|" * len(langs)]
    for t in task_ids:
        cells = []
        for l in langs:
            r = by.get((t, l))
            cells.append(SYMBOL.get(r["status"], "?") if r else "s")
        lines.append(f"| {t} | {tiers[t]} | " + " | ".join(cells) + " |")
    lines += ["", "### Pass rate by language and tier", "", "| tier | " + " | ".join(langs) + " |", "|---|" + "---|" * len(langs)]
    for tier in sorted(set(tiers.values())):
        row = []
        for l in langs:
            rs = [by[(t, l)] for t in task_ids if tiers[t] == tier and (t, l) in by]
            row.append(f"{sum(r['status'] == 'pass' for r in rs)}/{len(rs)}" if rs else "n/a")
        lines.append(f"| {tier} | " + " | ".join(row) + " |")
    row = []
    for l in langs:
        rs = [by[(t, l)] for t in task_ids if (t, l) in by]
        row.append(f"**{sum(r['status'] == 'pass' for r in rs)}/{len(rs)}**" if rs else "n/a")
    lines.append("| **all** | " + " | ".join(row) + " |")
    return lines


def gaps_section(results, lang):
    lines = [f"## {lang}: tasks that do not pass", ""]
    bad = [r for r in results if r["lang"] == lang and r["status"] not in ("pass", "missing")]
    if not bad:
        return lines + ["none", ""]
    lines += ["| task | status | passed | categories | first diagnostic |", "|---|---|---|---|---|"]
    for r in sorted(bad, key=lambda r: r["task"]):
        d = (r["build"]["diagnostics"][:1] or r["failures"][:1] or [{"message": ""}])[0]
        msg = d["message"].replace("|", "/").replace("\n", " ")[:110]
        lines.append(f"| {r['task']} | {r['status']} | {r['passed']}/{r['total']} | {', '.join(r['categories'])} | {msg} |")
    return lines + [""]


def agent_section(transcripts):
    if not transcripts:
        return ["## Agent runs", "", "No transcripts recorded.", ""]
    by = defaultdict(list)
    for t in transcripts:
        by[t["lang"]].append(t)
    lines = ["## Agent runs (from recorded transcripts)", "",
             "| lang | trials | pass@1 | within 2 iters | within 3 iters | mean iters to green | mean fix attempts | tokens in/out | wall s |",
             "|---|---|---|---|---|---|---|---|---|"]
    detail = {}
    for l in LANGS:
        if l not in by:
            continue
        a = metrics.aggregate(by[l])
        detail[l] = a
        w = a["pass_within_iterations"]
        tok = "n/a" if a["tokens_in"] is None else f"{a['tokens_in']}/{a['tokens_out']}"
        lines.append(f"| {l} | {a['trials']} | {a['pass_at_1']} | {w.get('2', 'n/a')} | {w.get('3', 'n/a')} | "
                     f"{a['mean_iterations_to_green']} | {a['mean_fix_attempts']} | {tok} | {a['wall_seconds']} |")
    lines += ["", "### Diagnostic categories encountered", "", "| lang | " + " | ".join(sorted({c for a in detail.values() for c in a['diagnostic_categories']})) + " |"]
    cats = sorted({c for a in detail.values() for c in a["diagnostic_categories"]})
    lines.append("|---|" + "---|" * len(cats))
    for l, a in detail.items():
        lines.append(f"| {l} | " + " | ".join(str(a["diagnostic_categories"].get(c, 0)) for c in cats) + " |")
    return lines + [""]


def build_report(baseline=None, transcripts=None):
    results = (baseline or {}).get("results", [])
    tc = (baseline or {}).get("toolchains", {})
    langs = [l for l in LANGS if any(r["lang"] == l for r in results)] or LANGS
    task_ids = sorted({r["task"] for r in results})
    md = ["# agent-dx report", ""]
    if tc:
        md += ["## Toolchains", ""] + [f"- {l}: {'OK' if i['available'] else 'SKIPPED'} {i.get('version', '')} {i.get('reason', '')}" for l, i in tc.items()] + [""]
    if results:
        md += matrix_section(results, langs, task_ids) + [""]
        for l in langs:
            if l.startswith("wj"):
                md += gaps_section(results, l)
    md += agent_section(transcripts or [])
    md += ["## Methodology limits", "",
           "See README.md: training-data contamination (Rust/Go/Python/TS are heavily represented in LLM training data, Windjammer is not), prompt variance, small trial counts, task-suite selection bias, and in-repo oracle visibility.", ""]
    js = {"toolchains": tc, "matrix": [{k: r[k] for k in ("task", "lang", "status", "passed", "total", "categories")} for r in results],
          "agent_aggregates": {}}
    by = defaultdict(list)
    for t in transcripts or []:
        by[t["lang"]].append(t)
    for l, ts in by.items():
        js["agent_aggregates"][l] = metrics.aggregate(ts)
    return "\n".join(md), js
