"""R7: stability. A cache file (.wj-readiness-history.json) records, per commit, whether R1-R6 passed on a
full run. R7 passes when each of the last N commits has a full-run entry with R1-R6 all passing. Nightly CI
must keep the file (see .github/workflows/readiness.yml: it is restored and re-uploaded every run)."""
import json, os, time
from .core import run, pct

N = 20


def load(path):
    try:
        with open(path) as f:
            return json.load(f).get("entries", [])
    except (OSError, ValueError):
        return []


def append(path, entries, commit, mode, criteria):
    st = {c["id"]: c["status"] for c in criteria if c["id"] in ("R1", "R2", "R3", "R4", "R5", "R6")}
    entry = {"commit": commit, "mode": mode, "time": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
             "criteria": st, "all_pass": all(v == "pass" for v in st.values()) and len(st) == 6}
    kept = [e for e in entries if not (e["commit"] == commit and e["mode"] == mode)]
    kept.append(entry)
    with open(path, "w") as f:
        json.dump({"note": "appended by scripts/interp_readiness.py; nightly CI must keep this file", "entries": kept[-400:]}, f, indent=1)
        f.write("\n")
    return kept


def last_commits(root, n=N, runner=run):
    r = runner(["git", "rev-list", "--first-parent", "-n", str(n), "HEAD"], cwd=root, timeout=30)
    return r.out.split() if r.rc == 0 else []


def evaluate(commits, entries, n=N):
    """R7 criterion from the commit list (newest first) and the history entries (pure)."""
    full = {e["commit"]: e for e in entries if e["mode"] == "full"}
    passed = [c for c in commits if full.get(c, {}).get("all_pass")]
    blockers = []
    for c in commits:
        e = full.get(c)
        if e is None:
            blockers.append({"program": c[:10], "reason": "no full readiness run recorded for this commit"})
        elif not e["all_pass"]:
            bad = ",".join(k for k, v in e["criteria"].items() if v != "pass")
            blockers.append({"program": c[:10], "reason": f"failed {bad}"})
    status = "pass" if len(commits) >= n and len(passed) == n else ("fail" if len(commits) < n or blockers else "pass")
    return {"id": "R7", "title": "Stability", "target": f"R1-R6 hold on the last {n} commits",
            "percent": pct(len(passed), n), "status": status,
            "detail": {"window": n, "commits_in_window": len(commits), "commits_passing": len(passed),
                       "commits_recorded": sum(1 for c in commits if c in full)},
            "blockers": blockers}
