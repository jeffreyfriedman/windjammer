"""Scorecard rendering (table + blockers) and the JSON document."""
import json, os, re

ORDER = ["R1", "R2", "R3", "R4", "R5", "R6", "R7"]
MARK = {"pass": "PASS", "fail": "FAIL", "unmeasured": "UNMEASURED"}


def overall(criteria, selfcheck, quick):
    if selfcheck["issues"]:
        return "SUSPECT"
    if all(c["status"] == "pass" for c in criteria) and len(criteria) == 7:
        return "READY-QUICK-SUBSET" if quick else "READY"
    return "NOT READY"


def exit_code(verdict):
    return {"READY": 0, "READY-QUICK-SUBSET": 0, "SUSPECT": 3}.get(verdict, 1)


def render(doc, top=5):
    rows = [f"{'':3}{'criterion':<34}{'status':<12}{'met':>7}   target"]
    for c in doc["criteria"]:
        p = "n/a" if c["percent"] is None else f"{c['percent']}%"
        rows.append(f"{c['id']:<3}{c['title']:<34}{MARK[c['status']]:<12}{p:>7}   {c['target']}")
    out = ["interpreter-primary readiness scorecard", f"commit {doc['commit'][:10]}  mode {doc['mode']}  load {doc['load_avg_1m']}  wj {doc['wj_profile']}", ""] + rows + [""]
    for c in doc["criteria"]:
        if c["status"] != "pass" and c["blockers"]:
            out.append(f"{c['id']} blockers ({len(c['blockers'])}):")
            out += [f"  {b['program']}: {b['reason']}" for b in c["blockers"][:top]]
            if len(c["blockers"]) > top:
                out.append(f"  ... {len(c['blockers']) - top} more (see JSON)")
    if doc["selfcheck"]["issues"]:
        out.append("SUSPECT measurement (cross-checks failed):")
        out += [f"  {i}" for i in doc["selfcheck"]["issues"]]
    out += ["", f"verdict: {doc['verdict']}"]
    if doc["mode"] == "quick":
        out.append("quick mode samples small subsets; only --full is a readiness verdict")
    return "\n".join(out)


def build_doc(commit, mode, load, profile, criteria, selfcheck, quick, generated):
    return {"schema_version": 1, "generated_at": generated, "commit": commit, "mode": mode, "load_avg_1m": load,
            "wj_profile": profile, "criteria": criteria, "selfcheck": selfcheck,
            "verdict": overall(criteria, selfcheck, quick)}


def assert_clean(text, forbidden):
    """Committed output must never carry a private project's name or path."""
    for f in forbidden:
        if f and f in text:
            raise SystemExit("refusing to write: output contains a configured private project name/path")
