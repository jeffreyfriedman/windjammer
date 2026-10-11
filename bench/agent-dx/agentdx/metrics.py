"""Per-trial and aggregate metrics (pure functions over transcript dicts)."""
import difflib
from math import comb


def pass_at_k(n, c, k):
    """Unbiased pass@k estimator (Chen et al. 2021): n samples, c correct, budget k."""
    if n - c < k:
        return 1.0
    return 1.0 - comb(n - c, k) / comb(n, k)


def changed_lines(a, b):
    sm = difflib.SequenceMatcher(None, a.splitlines(), b.splitlines(), autojunk=False)
    return sum(max(i2 - i1, j2 - j1) for tag, i1, i2, j1, j2 in sm.get_opcodes() if tag != "equal")


def build_events(trial):
    """Chronological build attempts: agent-run `check` calls and driver round builds."""
    ev = []
    for it in trial["iterations"]:
        for e in it.get("agent_checks", []):
            ev.append({"ok": e["build_ok"], "categories": e.get("categories", []), "source": e.get("source", ""), "iteration": it["index"]})
        ev.append({"ok": it["build"]["ok"], "categories": it["build"].get("categories", []),
                   "source": it.get("source", ""), "iteration": it["index"]})
    return ev


def first_error_to_fix(trial):
    """Distance from the first failing build to the next passing build.
    Returns {"attempts": n, "changed_lines": m} or None (never failed / never fixed)."""
    ev = build_events(trial)
    for i, e in enumerate(ev):
        if not e["ok"]:
            for j in range(i + 1, len(ev)):
                if ev[j]["ok"]:
                    return {"attempts": j - i, "changed_lines": changed_lines(e["source"], ev[j]["source"])}
            return None
    return None


def trial_metrics(trial):
    its = trial["iterations"]
    green = next((it["index"] for it in its if it["hidden"]["status"] == "pass"), None)
    cats = {}
    for e in build_events(trial):
        for c in e["categories"]:
            cats[c] = cats.get(c, 0) + 1
    for it in its:
        if it["hidden"]["status"] == "build_error":
            continue  # already counted through the build event
        for c in it["hidden"].get("categories", []):
            cats[c] = cats.get(c, 0) + 1
    tin = sum((it.get("usage") or {}).get("input_tokens", 0) for it in its)
    tout = sum((it.get("usage") or {}).get("output_tokens", 0) for it in its)
    reported = any(it.get("usage") for it in its)
    return {
        "green": green is not None,
        "pass_at_1": bool(its) and its[0]["hidden"]["status"] == "pass",
        "iterations_to_green": green,
        "iterations_used": len(its),
        "first_error_to_fix": first_error_to_fix(trial),
        "diagnostic_categories": cats,
        "tokens_in": tin if reported else None,
        "tokens_out": tout if reported else None,
        "wall_seconds": round(sum(it.get("wall_seconds", 0) for it in its), 2),
    }


def aggregate(trials):
    """Aggregate trials for one (lang, task-set). pass@k = fraction of tasks solved within k iterations."""
    ms = [trial_metrics(t) for t in trials]
    n = len(ms)
    if not n:
        return {"trials": 0}
    maxk = max(m["iterations_used"] for m in ms)
    within = {}
    for k in sorted({1, 2, 3, maxk}):
        within[str(k)] = round(sum(1 for m in ms if m["iterations_to_green"] and m["iterations_to_green"] <= k) / n, 3)
    greens = [m["iterations_to_green"] for m in ms if m["iterations_to_green"]]
    cats = {}
    for m in ms:
        for c, v in m["diagnostic_categories"].items():
            cats[c] = cats.get(c, 0) + v
    ttf = [m["first_error_to_fix"] for m in ms if m["first_error_to_fix"]]
    return {
        "trials": n, "pass_at_1": round(sum(m["pass_at_1"] for m in ms) / n, 3),
        "pass_within_iterations": within,
        "mean_iterations_to_green": round(sum(greens) / len(greens), 2) if greens else None,
        "mean_fix_attempts": round(sum(x["attempts"] for x in ttf) / len(ttf), 2) if ttf else None,
        "mean_fix_changed_lines": round(sum(x["changed_lines"] for x in ttf) / len(ttf), 1) if ttf else None,
        "tokens_in": sum(m["tokens_in"] or 0 for m in ms) if any(m["tokens_in"] is not None for m in ms) else None,
        "tokens_out": sum(m["tokens_out"] or 0 for m in ms) if any(m["tokens_out"] is not None for m in ms) else None,
        "wall_seconds": round(sum(m["wall_seconds"] for m in ms), 1),
        "diagnostic_categories": cats,
    }
