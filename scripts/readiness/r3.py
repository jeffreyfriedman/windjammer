"""R3: fallback is rare and loud. Counts default `wj run` invocations that switch to the compiled path
(the one-line stderr notice) plus interpreted tests the interpreter declined, grouped by reason."""
import os, re
from collections import Counter
from .core import file_hash, pct

TARGET_MAX = 2.0
NOTICE = re.compile(r"^wj: running compiled \(cargo\):\s*(.*?)(?: \([^()]*\))?; use `wj run --compiled` to skip", re.M)
ANY_NOTICE = re.compile(r"^wj: running compiled", re.M)


def normalize(reason):
    return re.sub(r"\d+", "N", reason)[:120]


def parse_notice(err):
    """(fell_back, reason, loud): `loud` is False when the notice carries no reason."""
    if not ANY_NOTICE.search(err):
        return False, "", True
    m = NOTICE.search(err)
    reason = normalize(m.group(1)) if m else ""
    return True, reason or "(no reason given)", bool(reason)


def measure(ctx, corpora, r1_rows):
    runs, falls, silent, reasons, blockers = 0, 0, [], Counter(), []
    limit = 6 if ctx.quick else 10**9
    for name, progs in corpora.items():
        for p in progs[:limit]:
            r = ctx.cached_run("r3-run", file_hash(p.path), [ctx.wj, "run", p.path], ctx.timeouts["interp"],
                               keep_timeouts=True, cwd=os.path.dirname(p.path),
                               env={"WJ_CARGO_TARGET_ROOT": ctx.target_root()})
            runs += 1
            fb, why, loud = parse_notice(r.err)
            if fb:
                falls += 1
                reasons[why] += 1
                blockers.append({"program": p.name, "reason": why})
                if not loud:
                    silent.append(p.name)
    for row in r1_rows:
        for u in row.get("unsupported", []):
            runs += 1
            falls += 1
            reasons[u["reason"]] += 1
            blockers.append(u)
        runs += row.get("interp_pass", 0)
    return summarize(runs, falls, silent, reasons, blockers)


def summarize(runs, falls, silent, reasons, blockers):
    rate = round(100.0 * falls / runs, 2) if runs else None
    ok = runs > 0 and rate <= TARGET_MAX and not silent
    blk = [dict(b, reason=f"fallback: {b['reason']}") for b in blockers] if not ok else []
    blk += [{"program": s, "reason": "fallback notice has no reason"} for s in silent]
    return {"id": "R3", "title": "Fallback is rare and loud", "target": f"<= {TARGET_MAX}% of invocations fall back",
            "percent": pct(runs - falls, runs), "status": "unmeasured" if not runs else ("pass" if ok else "fail"),
            "detail": {"invocations": runs, "fallbacks": falls, "fallback_rate_pct": rate, "silent": silent,
                       "reasons": dict(Counter(reasons).most_common())}, "blockers": blk}
