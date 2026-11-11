"""Pilot 6 report: R (Rust) vs U (Windjammer compiled-first) vs V2 (interpreted-first with predictions and `--verify`).

Reuses the pilot report (arm means, paired differences, per-task, categories) and adds: load-noise marking, the ADR-0033
acceptance table for V2, cost per correct program, the Rust compile-error tally, and the prediction precision check.
Python fallback: reuses the existing bootstrap and trial schema of this harness; a Windjammer port waits for the
harness to move (no wj blocker recorded)."""
import json
import os
import shutil
import tempfile
from collections import defaultdict

from . import arms as A, grade as G, pilot as PL, pilot_report as PR, stats

NOISY_LOAD = 16.0
ADR = {"pass": 0.95, "cost": 0.030, "tool": 1.0, "silent": 0.05, "load_pct": 0.10}


def noisy(t):
    return (t.get("load_avg") or 0) > NOISY_LOAD


def clusters(trials, arm, fn):
    d = defaultdict(list)
    for t in trials:
        if t["arm"] == arm:
            d[t["task"]].append(fn(t))
    return list(d.values())


def ci(trials, arm, fn, n_boot=2000):
    return stats.bootstrap_ci(clusters(trials, arm, fn), n_boot)


def fmt(c, nd=3):
    return "n/a" if c is None else f"{c[0]:.{nd}f} [{c[1]:.{nd}f}, {c[2]:.{nd}f}]"


def check_predictions(trials, say=print):
    """For every prediction shown to an agent, build that exact source compiled: accepted => the prediction was a false positive."""
    out = []
    for t in trials:
        for p in t.get("predictions") or []:
            d = tempfile.mkdtemp(prefix="p6prec-")
            try:
                open(os.path.join(d, "main.wj"), "w").write(p["source"])
                g = G.grade("wj", t["task"], d, hidden=False)
                row = {"task": t["task"], "arm": t["arm"], "trial": t["trial"], "call": p["call"], "build_status": g["status"],
                       "false_positive": g["status"] != "build_error"}
            finally:
                shutil.rmtree(d, ignore_errors=True)
            say(row)
            out.append(row)
    return out


def adr_table(trials, precision, n_boot=2000):
    v = [t for t in trials if t["arm"] == "V2"]
    if not v:
        return [], {}
    others = [t for t in trials if t["arm"] in ("U", "R")]
    mean = lambda xs: sum(xs) / len(xs) if xs else None
    pass_ci = ci(trials, "V2", lambda t: 1.0 if t["pass_final"] else 0.0, n_boot)
    cost_ci = ci(trials, "V2", lambda t: t["usage"].get("cost_usd"), n_boot)
    tool_all = ci(trials, "V2", lambda t: t["tool_wait_seconds"], n_boot)
    quiet = [t for t in v if not noisy(t)]
    tool_q = mean([t["tool_wait_seconds"] for t in quiet])
    sil = ci(trials, "V2", lambda t: 1.0 if t["divergence"]["silent"] else 0.0, n_boot)
    fp = [r for r in precision if r["arm"] == "V2" and r["false_positive"]]
    n_pred = sum(1 for r in precision if r["arm"] == "V2")
    load_v = mean([t["load_avg"] for t in v if t.get("load_avg") is not None])
    load_o = mean([t["load_avg"] for t in others if t.get("load_avg") is not None])
    rows, js = [], {}

    def row(name, bar, value, ok, note=""):
        rows.append(f"| {name} | {bar} | {value} | {'PASS' if ok else 'FAIL'} | {note} |")
        js[name] = {"bar": bar, "value": value, "pass": bool(ok)}
    row("pass@1 (compiled oracle)", ">= 0.95", fmt(pass_ci), pass_ci and pass_ci[0] >= ADR["pass"],
        "the ADR also asks V2 minus W to include zero; there is no W arm in this pilot (see the V2 minus U row in the paired table)")
    row("cost per trial, USD", "<= 0.030", fmt(cost_ci), cost_ci and cost_ci[0] <= ADR["cost"], "point estimate decides; interval shown")
    row("tool-wait, seconds", "<= 1.0", f"{tool_all[0]:.2f} all trials [{tool_all[1]:.2f}, {tool_all[2]:.2f}]; {('%.2f' % tool_q) if tool_q is not None else 'n/a'} on {len(quiet)} quiet trials (load <= {NOISY_LOAD:.0f})",
        tool_all[0] <= ADR["tool"], "includes the `--verify` build; load inflates it")
    row("silent divergence", "<= 0.05", fmt(sil), sil and sil[0] <= ADR["silent"])
    row("no prediction for a program the build accepts", "0 false positives", f"{len(fp)} of {n_pred} predictions shown to V2 agents", not fp,
        "each shown prediction's exact source rebuilt compiled" if n_pred else "no V2 trial was shown a prediction, so this criterion is vacuous")
    ratio = None if not (load_v and load_o) else abs(load_v - load_o) / load_o
    row("machine load within 10% of the comparison arm", "within 10%", f"V2 {load_v:.1f} vs U/R {load_o:.1f}" if ratio is not None else "n/a",
        ratio is not None and ratio <= ADR["load_pct"], "the V arm of pilot 5 is not re-run; U and R of this pilot are the comparison")
    return rows, js


def build(trials, precision, n_boot=2000):
    md, js = PR.build_report(trials, n_boot=n_boot)
    md = md.replace("# Agent preference pilot", "# Pilot 6: Rust vs Windjammer compiled-first vs Windjammer interpreted-first with predictions", 1)
    extra = ["", "## Cost and pass comparison: R, U, V2", "",
             "Cost per correct program = mean cost per trial / pass@1 (a failed trial is assumed redone in full; an upper-bound convention, as in ADR-0033).", "",
             "| arm | trials | pass@1 | cost per trial USD | cost per correct program USD | turns | tokens out |", "|---|---|---|---|---|---|---|"]
    js["comparison"] = {}
    for a in ("R", "U", "V2"):
        ts = [t for t in trials if t["arm"] == a]
        if not ts:
            continue
        p = ci(trials, a, lambda t: 1.0 if t["pass_final"] else 0.0, n_boot)
        c = ci(trials, a, lambda t: t["usage"].get("cost_usd"), n_boot)
        tu = ci(trials, a, lambda t: t["usage"].get("turns"), n_boot)
        to = ci(trials, a, lambda t: t["usage"].get("tokens_out"), n_boot)
        cpc = c[0] / p[0] if p and p[0] > 0 else None
        js["comparison"][a] = {"pass": p, "cost": c, "cost_per_correct": cpc}
        extra.append(f"| {a} {A.ARMS[a]['name']} | {len(ts)} | {fmt(p)} | {fmt(c)} | {('%.3f' % cpc) if cpc else 'n/a'} | {fmt(tu, 1)} | {fmt(to, 0)} |")
    # noisy trials
    extra += ["", "## Machine load and noisy trials", "",
              f"A trial is noisy when its mean load average exceeded {NOISY_LOAD:.0f}. Cost, tokens, turns and pass are not affected by load; wall and tool-wait are.", "",
              "| arm | trials | noisy | wall s (all) | wall s (quiet only) | tool-wait s (all) | tool-wait s (quiet only) | mean load |", "|---|---|---|---|---|---|---|---|"]
    js["noisy"] = {}
    for a in ("R", "U", "V2"):
        ts = [t for t in trials if t["arm"] == a]
        if not ts:
            continue
        q = [t for t in ts if not noisy(t)]
        m = lambda xs: (sum(xs) / len(xs)) if xs else None
        f = lambda x: "n/a" if x is None else f"{x:.1f}"
        row = {"trials": len(ts), "noisy": len(ts) - len(q), "wall_all": m([t["wall_seconds"] for t in ts]), "wall_quiet": m([t["wall_seconds"] for t in q]),
               "tool_all": m([t["tool_wait_seconds"] for t in ts]), "tool_quiet": m([t["tool_wait_seconds"] for t in q]),
               "load": m([t["load_avg"] for t in ts if t.get("load_avg") is not None])}
        js["noisy"][a] = row
        extra.append(f"| {a} | {len(ts)} | {len(ts) - len(q)} | {f(row['wall_all'])} | {f(row['wall_quiet'])} | {f(row['tool_all'])} | {f(row['tool_quiet'])} | {f(row['load'])} |")
    # Rust compile errors
    r = [t for t in trials if t["arm"] == "R"]
    if r:
        m = lambda k: sum(t.get(k) or 0 for t in r) / len(r)
        hit = sum(1 for t in r if (t.get("compile_errors") or 0) > 0)
        extra += ["", "## Arm R: rustc errors the agent hit", "",
                  f"{hit}/{len(r)} trials saw at least one rustc error. Mean errors per trial {m('compile_errors'):.2f}; mean failing rustc calls {m('failing_verify_calls'):.2f}; "
                  f"mean rustc calls spent from the first failure to the next success {m('fix_calls'):.2f} (a proxy for turns spent fixing).", ""]
        js["rust_errors"] = {"trials_with_errors": hit, "n": len(r), "errors_per_trial": m("compile_errors"), "fix_calls_per_trial": m("fix_calls")}
    rows, adr = adr_table(trials, precision, n_boot)
    if rows:
        extra += ["", "## ADR-0033 acceptance table (arm V2)", "", "| criterion | bar | measured | verdict | note |", "|---|---|---|---|---|"] + rows
        js["adr0033"] = adr
        vs = [t for t in trials if t["arm"] == "V2"]
        extra += ["", f"V2 trials that ran `wj run --verify`: {sum(1 for t in vs if t.get('verify_used'))}/{len(vs)}. "
                  f"V2 trials shown a prediction: {sum(1 for t in vs if t.get('prediction_shown'))}/{len(vs)}; "
                  f"U trials shown one: {sum(1 for t in trials if t['arm'] == 'U' and t.get('prediction_shown'))}."]
    if precision:
        js["precision"] = precision
    return md + "\n".join(extra) + "\n", js
