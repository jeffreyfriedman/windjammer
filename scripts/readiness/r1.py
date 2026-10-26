"""R1: per-test pass/fail sets under `wj test --interpret` vs compiled `wj test`, per project."""
import json, os, re, shutil
from .core import pct, scrub, tree_hash, sha

TARGET = 99.0
_TEST_LINE = re.compile(r'\{"name":\s*"([^"]*)",\s*"file":\s*"([^"]*)",\s*"status":\s*"([^"]*)"\}')


def _tid(project_path, file, name):
    rel = os.path.relpath(file, project_path) if os.path.isabs(file) else file
    rel = rel[2:] if rel.startswith("./") else rel
    return f"{rel}::{name.split('::')[-1]}"


def parse_tests(out, project_path):
    """{test id: bool passed}, or None when the output carries no per-test list. Accepts the interpreted
    JSON document, the compiled `--json` stream, and libtest-style `test X ... ok` lines."""
    res = {}
    start = out.find("{")
    if start >= 0:
        try:
            doc = json.loads(out[start:out.rindex("}") + 1])
            for t in doc.get("tests", []):
                ok = t["passed"] if "passed" in t else t.get("status") == "passed"
                res[_tid(project_path, t.get("file", ""), t["name"])] = bool(ok)
        except (ValueError, KeyError, TypeError):
            res = {}
    if not res:
        for name, file, status in _TEST_LINE.findall(out):
            res[_tid(project_path, file, name)] = status == "passed"
    if not res:
        for m in re.finditer(r"^test (\S+) \.\.\. (ok|FAILED)", out, re.M):
            res[m.group(1)] = m.group(2) == "ok"
    return res or None


def run_project(ctx, proj):
    ph = tree_hash(proj.path)
    t = ctx.timeouts
    i = ctx.cached_run("r1-interp", sha(ph, proj.label), [ctx.wj, "test", "--interpret", "--json", proj.path],
                       t["interp"], cwd=proj.path)
    scratch = sha(proj.label, ph)[:12] if ctx.cold else None
    root = ctx.target_root(scratch)
    c = ctx.cached_run("r1-compiled" + ("-cold" if ctx.cold else "-warm"), sha(ph, proj.label), [ctx.wj, "test", "--json", proj.path],
                       t["compiled"], cwd=proj.path, env={"WJ_CARGO_TARGET_ROOT": root})
    if '"status": "unknown"' in c.out and c.rc != 0 and not c.timed_out:
        # --json hides cargo's diagnostics; replay once (plain output) to learn why the test crate did not build
        d = ctx.cached_run("r1-compiled-why", sha(ph, proj.label), [ctx.wj, "test", proj.path], t["compiled"], cwd=proj.path,
                           env={"WJ_CARGO_TARGET_ROOT": root})
        c.why = next((l.strip() for l in (d.out + "\n" + d.err).splitlines() if l.startswith("error")), "")
    if scratch:
        shutil.rmtree(root, ignore_errors=True)  # a cold build leaves gigabytes behind
    return i, c


def evaluate(proj, interp, comp, paths, cold=False):
    """Pure scorecard logic: compare the two result sets of one project."""
    ti, tc = parse_tests(interp.out, proj.path), parse_tests(comp.out, proj.path)
    row = {"project": proj.label, "interp_secs": round(interp.secs, 1), "compiled_secs": round(comp.secs, 1),
           "measured": True, "compiled_cache": "cold" if cold else "warm"}
    if tc is not None and not any(tc.values()) and comp.rc != 0 and '"status": "unknown"' in comp.out:
        tc = None  # every test unknown: the compiled test crate did not build or run, so there is nothing to compare with
        broke = "compiled test run failed before reporting results: " + (scrub(getattr(comp, "why", ""), paths) or "build error in the generated test crate")
    else:
        broke = None
    if tc is None:
        row["measured"] = False
        row.update(status="unmeasured", reason=broke or ("compiled run timed out" if comp.timed_out else
                   "compiled run produced no test list: " + scrub(comp.err or comp.out, paths)))
        row.update(compiled_pass=0, both_pass=0, mismatches=[])
        return row
    ti = ti or {}
    cpass = {k for k, v in tc.items() if v}
    ipass = {k for k, v in ti.items() if v}
    bad = sorted(cpass - ipass)                # passes compiled, not interpreted
    extra = sorted(ipass - cpass)              # passes interpreted, fails/absent compiled (suspicious)
    msgs = _messages(interp.out, proj.path)
    no_list = ti == {} and cpass
    why = "interpreted run produced no test list: " + scrub(interp.err or interp.out, paths) if no_list else ""
    unsup = _unsupported(interp.out, proj)
    row.update(status="ok", sets_equal=set(ti) == set(tc), unsupported=unsup, compiled_total=len(tc), compiled_pass=len(cpass), both_pass=len(cpass & ipass),
               interp_pass=len(ipass), mismatches=[{"program": f"{proj.label}:{_anon(proj, b)}", "reason": why or scrub(msgs.get(b, ""), paths) or "fails interpreted, passes compiled"} for b in bad],
               interp_only_pass=[_anon(proj, e) for e in extra])
    return row


_UNSUP = re.compile(r"not interpreted|unsupported|not supported|use `wj test`|not running", re.I)


def _unsupported(out, proj):
    """Interpreted failures that are the interpreter declining (not a wrong answer): R3 counts them as fallbacks."""
    try:
        doc = json.loads(out[out.find("{"):out.rindex("}") + 1])
    except ValueError:
        return []
    return [{"program": f"{proj.label}:{_anon(proj, _tid(proj.path, t.get('file', ''), t['name']))}", "reason": _norm_reason(t.get("message", ""))}
            for t in doc.get("tests", []) if not t.get("passed", True) and _UNSUP.search(t.get("message", ""))]


def _norm_reason(msg):
    first = msg.strip().splitlines()[0] if msg.strip() else ""
    return re.sub(r"\d+", "N", first)[:120]


def _anon(proj, tid):
    return sha(tid)[:10] if proj.private else tid


def _messages(out, project_path):
    """{test id: failure message} from the interpreted JSON document."""
    try:
        doc = json.loads(out[out.find("{"):out.rindex("}") + 1])
        return {_tid(project_path, t.get("file", ""), t["name"]): t.get("message", "") for t in doc.get("tests", [])}
    except (ValueError, KeyError):
        return {}


def measure(ctx):
    paths = [(p.path, f"<{p.label}>") for p in ctx.projects]
    rows = []
    for proj in ctx.projects:
        i, c = run_project(ctx, proj)
        rows.append(evaluate(proj, i, c, paths, ctx.cold))
    return summarize(rows)


def summarize(rows):
    meas = [r for r in rows if r["measured"]]
    den = sum(r["compiled_pass"] for r in meas)
    num = sum(r["both_pass"] for r in meas)
    p = pct(num, den)
    blockers = [m for r in meas for m in r["mismatches"]]
    unmeasured = [r for r in rows if not r["measured"]]
    blockers += [{"program": r["project"], "reason": "unmeasured: " + r["reason"]} for r in unmeasured]
    if not rows:
        status = "unmeasured"
    elif unmeasured or p is None:
        status = "fail" if (p is not None and p < TARGET) else "unmeasured"
    else:
        status = "pass" if p >= TARGET else "fail"
    return {"id": "R1", "title": "Test suites pass interpreted", "target": f">= {TARGET}% of compiled-passing tests",
            "percent": p, "status": status, "detail": {"projects": rows, "tests_compared": den, "tests_agree": num},
            "blockers": blockers}
