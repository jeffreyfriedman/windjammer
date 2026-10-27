"""Markdown + JSON report for pilot trials (schema 2): arm means with bootstrap CIs, per-task table,
failure categories, silent-divergence cost, and the ranked friction list."""
from collections import Counter, defaultdict

from . import aps as APS, friction, stats
from .arms import ARMS

METRICS = [  # key, label, extractor(trial) -> number|None
    ("pass", "pass@1 (compiled hidden oracle)", lambda t: 1.0 if t["pass_final"] else 0.0),
    ("verify", "verify calls (wj check/run/test/build)", lambda t: t["verify_calls"]),
    ("to_green", "verify calls to green (passing trials)", lambda t: t["verify_calls_to_green"] if t["pass_final"] else None),
    ("turns", "agent turns", lambda t: t["usage"].get("turns")),
    ("tok_in", "tokens in (incl. cache)", lambda t: t["usage"].get("tokens_in")),
    ("tok_out", "tokens out", lambda t: t["usage"].get("tokens_out")),
    ("cost", "cost USD", lambda t: t["usage"].get("cost_usd")),
    ("wall", "wall seconds", lambda t: t["wall_seconds"]),
    ("think", "agent-thinking seconds (API time)", lambda t: t.get("thinking_seconds")),
    ("tool", "tool-wait seconds (wj commands)", lambda t: t["tool_wait_seconds"]),
    ("denied", "harness permission denials (should be 0)", lambda t: t["usage"].get("permission_denials")),
    ("capden", "capability denials seen (WJ004x)", lambda t: t.get("capability_denials")),
    ("silent", "silent divergence (interp pass, compiled fail)", lambda t: 1.0 if t["divergence"]["silent"] else 0.0),
]


def by_arm_task(trials, fn):
    d = defaultdict(lambda: defaultdict(list))
    for t in trials:
        d[t["arm"]][t["task"]].append(fn(t))
    return d


def _fmt(ci, nd=2):
    if ci is None:
        return "n/a"
    m, lo, hi = ci
    return f"{m:.{nd}f} [{lo:.{nd}f}, {hi:.{nd}f}]"


def build_report(trials, n_boot=2000, prev=None):
    arms = sorted({t["arm"] for t in trials})
    js = {"n_trials": len(trials), "arms": {}, "diffs": {}, "friction": friction.build([t for t in trials if t.get("lang", "wj") == "wj"])}
    md = ["# Agent preference pilot", "",
          f"{len(trials)} trials over {len({t['task'] for t in trials})} tasks. Intervals are 95% bootstrap CIs "
          "that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.", ""]
    n = Counter(t["arm"] for t in trials)
    md += ["## Arms", "", "| arm | name | trials | context pack |", "|---|---|---|---|"]
    for a in arms:
        md.append(f"| {a} | {ARMS[a]['name']} | {n[a]} | {'yes' if ARMS[a]['pack'] else 'no'} |")
    md += ["", "## Arm means", "", "| metric | " + " | ".join(arms) + " |", "|---|" + "---|" * len(arms)]
    for key, label, fn in METRICS:
        row, cells = {}, []
        for a in arms:
            clusters = list(by_arm_task([t for t in trials if t["arm"] == a], fn)[a].values())
            ci = stats.bootstrap_ci(clusters, n_boot)
            row[a] = ci
            cells.append(_fmt(ci, 3 if key in ("pass", "silent", "cost") else 1))
        js["arms"][key] = row
        md.append(f"| {label} | " + " | ".join(cells) + " |")
    pairs = [(x, y) for i, x in enumerate(arms) for y in arms[i + 1:]]
    if pairs:
        md += ["", "## Paired differences (task-paired, first minus second)", "", "| metric | " + " | ".join(f"{x}-{y}" for x, y in pairs) + " |",
               "|---|" + "---|" * len(pairs)]
        for key, label, fn in METRICS:
            d = by_arm_task(trials, fn)
            cells = []
            for x, y in pairs:
                ci = stats.paired_diff_ci(d[x], d[y], n_boot)
                js["diffs"].setdefault(key, {})[f"{x}-{y}"] = ci
                cells.append(_fmt(ci, 3 if key in ("pass", "silent", "cost") else 1))
            md.append(f"| {label} | " + " | ".join(cells) + " |")
    md += ["", "## Per-task pass rate (passes/trials)", "", "| task | " + " | ".join(arms) + " |", "|---|" + "---|" * len(arms)]
    for task in sorted({t["task"] for t in trials}):
        cells = []
        for a in arms:
            ts = [t for t in trials if t["task"] == task and t["arm"] == a]
            cells.append(f"{sum(t['pass_final'] for t in ts)}/{len(ts)}" if ts else "-")
        md.append(f"| {task} | " + " | ".join(cells) + " |")
    md += ["", "## Top failure categories (diagnostics hit, all verify calls)", "", "| category | " + " | ".join(arms) + " |", "|---|" + "---|" * len(arms)]
    cats = {a: Counter() for a in arms}
    for t in trials:
        cats[t["arm"]].update(t["diagnostic_categories"])
    allc = sorted({c for a in arms for c in cats[a]}, key=lambda c: -sum(cats[a][c] for a in arms))
    for c in allc:
        md.append(f"| {c} | " + " | ".join(str(cats[a][c]) for a in arms) + " |")
    div = [t for t in trials if t["divergence"]["diverged"]]
    md += ["", "## Interpreter vs compiled divergence", "",
           f"{len(div)} of {len(trials)} trials had an interpreted hidden-oracle result different from the compiled one "
           f"({sum(t['divergence']['silent'] for t in trials)} silent: interpreter green, compiled red)."]
    for t in div[:10]:
        md.append(f"- {t['task']} arm {t['arm']} trial {t['trial']}: interp={t['divergence']['interp_pass']} compiled={t['divergence']['compiled_pass']}")
    base_arms = [a for a in arms if ARMS[a]["lang"] != "wj"]
    wj_arms = [a for a in arms if ARMS[a]["lang"] == "wj"]
    js["aps"] = {}
    if base_arms and wj_arms:
        md += ["", "## Agent preference score (Windjammer arm vs baseline language)", "",
               "APS = 100 * P^0.4 * I^0.2 * T^0.2 * W^0.2 (ratios baseline/Windjammer, clipped to [0.25, 4]; I = agent turns). 100 = parity; "
               "`P-limited` when pass@1 is below 0.9 of the baseline's. Components are the diagnosis; CIs are task-clustered bootstrap.", "",
               "| arm | vs | APS | P | I (turns) | T (tokens) | W (wall) |", "|---|---|---|---|---|---|---|"]
        for a in wj_arms:
            for b in base_arms:
                r = APS.aps([t for t in trials if t["arm"] == a], [t for t in trials if t["arm"] == b], max(200, n_boot // 2))
                js["aps"][f"{a}-vs-{b}"] = r
                if not r:
                    md.append(f"| {a} {ARMS[a]['name']} | {b} {ARMS[b]['name']} | n/a | | | | |")
                    continue
                f = lambda k: ("%.2f" % r["point"][k] + (" [%.2f, %.2f]" % r["ci"][k] if r["ci"].get(k) else ""))
                score = ("%.0f" % r["point"]["APS"] + (" [%.0f, %.0f]" % r["ci"]["APS"] if r["ci"].get("APS") else "")) if r["point"]["APS"] is not None else "P-limited"
                md.append(f"| {a} {ARMS[a]['name']} | {b} {ARMS[b]['name']} | {score} | {f('P')} | {f('I')} | {f('T')} | {f('W')} |")
    if prev:
        md += ["", "## Deltas versus the previous pilot (same tasks and arms; this run minus previous)", "",
               "| arm | metric | previous | this run | delta |", "|---|---|---|---|---|"]
        for a in sorted({t["arm"] for t in prev} & set(arms)):
            for key, label, fn in METRICS:
                if key == "denied":
                    continue
                p0 = stats.bootstrap_ci(list(by_arm_task([t for t in prev if t["arm"] == a], fn)[a].values()), 500)
                p1 = stats.bootstrap_ci(list(by_arm_task([t for t in trials if t["arm"] == a], fn)[a].values()), 500)
                if p0 and p1:
                    nd = 3 if key in ("pass", "silent", "cost") else 1
                    md.append(f"| {a} | {label} | {p0[0]:.{nd}f} | {p1[0]:.{nd}f} | {p1[0] - p0[0]:+.{nd}f} |")
                    js.setdefault("deltas", {}).setdefault(a, {})[key] = p1[0] - p0[0]
    viol = sum(t["mode_violations"] for t in trials)
    md += ["", f"Mode violations (agent used the other arm's loop commands): {viol}.", "", "## Friction list (ranked by estimated tokens wasted)", "",
           "| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |", "|---|---|---|---|---|---|---|---|---|"]
    for it in js["friction"][:25]:
        md.append(f"| {it['rank']} | {it['priority']} | {it['title']} (`{it['id']}`) | {it['fix_kind']} | {it['fix_location']} | {it['n_trials']} | {it['n_tasks']} | {it['wasted_calls']} | {it['est_tokens']} |")
    return "\n".join(md) + "\n", js
