"""R5: diagnostics agree. `wj check` and the interpreter reject the same programs, and the compiled
path (wj build + rustc metadata check) accepts everything the interpreter accepts. Mirrors the
interpreter_check_parity and check_build_differential gates natively; --cargo-gates runs the real ones too."""
import glob, os, shutil, tempfile
from .core import run, file_hash, pct
from . import pilot


def refused(err):
    return "not running" in err and "`wj check` reports" in err


def judge(name, check_ok, run_ok, run_out, run_err):
    """Parity verdict for one program (same rule as tests/interpreter_check_parity_test.rs `judge`)."""
    ref = refused(run_err)
    if not check_ok and (run_ok or run_out or not ref):
        return f"`wj check` rejects it but the interpreter did not refuse it (ok={run_ok}, stdout={run_out[:30]!r})"
    if check_ok and ref:
        return "`wj check` accepts it but the interpreter refused it"
    return None


def programs(root, quick):
    valid = sorted(glob.glob(os.path.join(root, "examples/verified/*.wj")))
    bad = sorted(glob.glob(os.path.join(root, "tests/check_negative/mutants/*.wj")))
    if quick:
        valid, bad = valid[:: max(len(valid) // 6, 1)][:6], bad[:: max(len(bad) // 10, 1)][:10]
        gaps = [f for f in sorted(glob.glob(os.path.join(root, "examples/verified/*.wj"))) if rust_gap(f)][:3]
        valid = sorted(set(valid) | set(gaps))
    return [(f, True) for f in valid], [(f, False) for f in bad]


def rust_gap(path):
    """The `// rust: gap <why>` header of a verified example: the compiled path is documented not to accept it yet."""
    try:
        for line in open(path, errors="replace"):
            if line.startswith("// rust: gap"):
                return line[len("// rust: gap"):].strip()
            if not line.startswith("//"):
                break
    except OSError:
        pass
    return None


def pipeline(ctx, path):
    """(accepted, first error line): `wj build --no-cargo`, then rustc's type/borrow check of the emitted Rust."""
    out = tempfile.mkdtemp(prefix="wj-r5-")
    try:
        b = ctx.runner([ctx.wj, "build", "--no-cargo", "--no-lint", "-o", out, path], timeout=ctx.timeouts["build"], cwd=out)
        rs = os.path.join(out, os.path.splitext(os.path.basename(path))[0] + ".rs")
        if b.rc != 0 or not os.path.exists(rs):
            return False, next((l.strip() for l in (b.err + b.out).splitlines() if "error" in l), "wj build failed")
        r = ctx.runner(["rustc", "--edition", "2021", "-A", "warnings", "--emit=metadata", "--crate-type=bin", "-o",
                        os.path.join(out, "m.rmeta"), rs], timeout=ctx.timeouts["build"], cwd=out)
        return r.rc == 0, next((l.strip() for l in r.err.splitlines() if l.startswith("error")), "")
    finally:
        shutil.rmtree(out, ignore_errors=True)


def pipeline_accepts(ctx, path):
    return pipeline(ctx, path)[0]


def measure(ctx, cargo_gates=False):
    valid, bad = programs(ctx.root, ctx.quick)
    blockers, checked, agree, build_checked, stale_gaps = [], 0, 0, 0, 0
    for path, _ in valid + bad:
        h = file_hash(path)
        name = os.path.relpath(path, ctx.root)
        c = ctx.cached_run("r5-check", h, [ctx.wj, "check", path], ctx.timeouts["check"], cwd=os.path.dirname(path))
        r = ctx.cached_run("r5-run", h, [ctx.wj, "run", "--interpret", path], ctx.timeouts["interp"], cwd=os.path.dirname(path))
        if c.timed_out or r.timed_out:
            blockers.append({"program": name, "reason": "unmeasured: timed out"})
            continue
        checked += 1
        v = judge(name, c.rc == 0, r.rc == 0, r.out, r.err)
        if v:
            blockers.append({"program": name, "reason": "[check-vs-interpreter] " + v})
        else:
            agree += 1
    # compiled path accepts what the interpreter accepts: valid programs only (the mutants are rejected by design)
    build_ok = 0
    for path, _ in valid:
        name = os.path.relpath(path, ctx.root)
        build_checked += 1
        gap = rust_gap(path)
        if pipeline_accepts(ctx, path):
            build_ok += 1
            stale_gaps += bool(gap)
        else:
            why = f" (documented gap: {gap[:70]})" if gap else " (not a documented gap)"
            blockers.append({"program": name, "reason": "[check-vs-build] interpreter accepts it, build + rustc rejects it" + why})
    pilot_rows = pilot.measure(ctx)["rows"]
    for r in pilot_rows:
        if r["silent"] and r["check_accepts"]:
            blockers.append({"program": r["program"], "reason": f"[agent-pilot check-miss] `wj check` accepts it, interpreter-green, compiled fails: {r['rustc_error'][:90]}"})
    gates = run_cargo_gates(ctx) if cargo_gates else None
    if gates and not gates["ok"]:
        blockers.append({"program": "cargo gates", "reason": gates["summary"]})
    real = [b for b in blockers if not b["reason"].startswith("unmeasured")]
    status = "fail" if real else ("unmeasured" if blockers or not checked else "pass")
    return {"id": "R5", "title": "Diagnostics agree", "target": "0 check/interpreter/build disagreements",
            "percent": pct(agree + build_ok, checked + build_checked), "status": status,
            "detail": {"parity_checked": checked, "parity_agree": agree, "build_checked": build_checked,
                       "negative_programs": len(bad), "gap_headers_now_accepted": stale_gaps, "cargo_gates": gates, "agent_pilot_check_misses": sum(1 for r in pilot_rows if r["silent"] and r["check_accepts"])},
            "blockers": blockers}


def run_cargo_gates(ctx):
    r = ctx.runner(["cargo", "test", "--test", "all", "--", "interpreter_check_parity", "check_build_differential"],
                   timeout=3600, cwd=ctx.root)
    return {"ok": r.rc == 0, "summary": (r.err.strip().splitlines() or [""])[-1][:160] if r.rc else "green"}
