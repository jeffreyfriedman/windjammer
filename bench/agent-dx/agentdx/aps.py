"""Agent preference score (docs/design/agent-preference.md): language L against baseline B on the same tasks.

APS = 100 * P^0.4 * I^0.2 * T^0.2 * W^0.2, each ratio clipped to [0.25, 4].
P = pass@1(L)/pass@1(B); I, T, W = baseline over L for agent turns, tokens (in+out) and wall seconds, each averaged over
PASSING trials only (failing fast earns nothing). Iterations are agent turns because they are comparable across toolchains.
When P < 0.9 the combined score is reported as None ("P-limited") and the components are still returned.
"""
import random

W_P, W_I, W_T, W_W = 0.4, 0.2, 0.2, 0.2
LO, HI = 0.25, 4.0


def _clip(x):
    return max(LO, min(HI, x))


def _mean(xs):
    return sum(xs) / len(xs) if xs else None


def _tok(t):
    u = t["usage"]
    return None if u.get("tokens_in") is None else u["tokens_in"] + (u.get("tokens_out") or 0)


def components(tl, tb):
    """tl, tb: lists of schema-2 trials of the same tasks. -> dict or None when undefined."""
    if not tl or not tb:
        return None
    pl, pb = sum(t["pass_final"] for t in tl) / len(tl), sum(t["pass_final"] for t in tb) / len(tb)
    if pb == 0 or pl == 0:
        return None
    gl, gb = [t for t in tl if t["pass_final"]], [t for t in tb if t["pass_final"]]
    def ratio(f):
        a, b = _mean([f(t) for t in gb if f(t) is not None]), _mean([f(t) for t in gl if f(t) is not None])
        return None if not a or not b else a / b
    r = {"P": pl / pb, "I": ratio(lambda t: t["usage"].get("turns")), "T": ratio(_tok), "W": ratio(lambda t: t["wall_seconds"])}
    if any(v is None for v in r.values()):
        return None
    r = {k: _clip(v) for k, v in r.items()}
    r["APS"] = None if r["P"] < 0.9 else 100 * r["P"] ** W_P * r["I"] ** W_I * r["T"] ** W_T * r["W"] ** W_W
    return r


def aps(tl, tb, n_boot=1000, seed=0, alpha=0.05):
    """Point estimate and task-clustered bootstrap CIs for each component and the score."""
    point = components(tl, tb)
    if point is None:
        return None
    tasks = sorted({t["task"] for t in tl} & {t["task"] for t in tb})
    byl = {k: [t for t in tl if t["task"] == k] for k in tasks}
    byb = {k: [t for t in tb if t["task"] == k] for k in tasks}
    rng = random.Random(seed)
    draws = {k: [] for k in ("P", "I", "T", "W", "APS")}
    for _ in range(n_boot):
        pick = [tasks[rng.randrange(len(tasks))] for _ in tasks]
        c = components([t for k in pick for t in byl[k]], [t for k in pick for t in byb[k]])
        if c:
            for k in draws:
                if c[k] is not None:
                    draws[k].append(c[k])
    out = {"point": point, "n_tasks": len(tasks), "ci": {}}
    for k, v in draws.items():
        v.sort()
        out["ci"][k] = (v[int(alpha / 2 * len(v))], v[min(len(v) - 1, int((1 - alpha / 2) * len(v)))]) if len(v) >= 20 else None
    return out
