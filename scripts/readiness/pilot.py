"""Agent-preference pilot corpus (bench/agent-dx/results/pilot): programs an agent wrote interpreted-first.
A program that passes the interpreter but fails the compiled grade is a silent divergence; if `wj check` also
accepts it, the parity failure is only discovered by rustc. Both are scorecard metrics (R2 and R5)."""
import glob, json, os, re, shutil, tempfile
from .core import sha

QUICK_SILENT, QUICK_OTHER = 4, 2


def load(root):
    out = []
    for f in sorted(glob.glob(os.path.join(root, "bench/agent-dx/results/pilot/trials/*.json"))):
        try:
            t = json.load(open(f))
        except ValueError:
            continue
        if t.get("arm") != "A" or not t.get("final_source"):
            continue
        d = t.get("divergence", {})
        out.append({"program": f"pilot/{t['task']}.A.{t['trial']}", "source": t["final_source"],
                    "interp_green": bool(d.get("interp_pass")), "silent": bool(d.get("silent"))})
    return out


def error_class(line):
    m = re.search(r"error(\[E\d+\])?", line)
    code = m.group(1) if m and m.group(1) else ""
    return code or "other"


def measure(ctx):
    """Cached per context: rows for the sampled/all pilot programs plus the headline rate over the full pilot."""
    if getattr(ctx, "_pilot", None) is not None:
        return ctx._pilot
    progs = load(ctx.root)
    green = [p for p in progs if p["interp_green"]]
    silent_all = [p for p in green if p["silent"]]
    pick = progs
    if ctx.quick:
        pick = [p for p in green if p["silent"]][:QUICK_SILENT] + [p for p in green if not p["silent"]][:QUICK_OTHER]
    gdir = os.path.join(tempfile.gettempdir(), "wj-readiness-pilot")  # outside the checkout (project-root detection)
    os.makedirs(gdir, exist_ok=True)
    rows = []
    for p in pick:
        if not p["interp_green"]:
            continue
        path = os.path.join(gdir, re.sub(r"[^A-Za-z0-9_]", "_", p["program"]) + ".wj")
        with open(path, "w") as f:
            f.write(p["source"])
        h = sha(p["source"])
        c = ctx.cached_run("pilot-check", h, [ctx.wj, "check", path], ctx.timeouts["check"], cwd=gdir)
        # a real cargo build: these programs use the runtime crate, which a bare rustc check cannot resolve
        out = os.path.join(gdir, "out_" + h[:10])
        b = ctx.cached_run("pilot-build-compiled", h, [ctx.wj, "build", path, "-o", out], ctx.timeouts["compiled"], cwd=gdir,
                           env={"WJ_CARGO_TARGET_ROOT": ctx.target_root()})
        shutil.rmtree(out, ignore_errors=True)  # each build leaves a cargo target tree behind
        ok = b.rc == 0
        err = "" if ok else ("unmeasured: " + b.err.strip()[:60] if b.timed_out else
                             next((l.strip() for l in (b.err + b.out).splitlines() if l.startswith("error[") or l.startswith("error:")), "build failed"))
        rows.append({"program": p["program"], "silent": p["silent"], "check_accepts": c.rc == 0,
                     "pipeline_accepts": ok, "rustc_error": err})
    ctx._pilot = {"rows": rows, "interp_green": len(green), "silent": len(silent_all), "total": len(progs)}
    return ctx._pilot


def summary(ctx):
    m = measure(ctx)
    classes = {}
    for r in m["rows"]:
        if r["silent"]:
            classes[error_class(r["rustc_error"])] = classes.get(error_class(r["rustc_error"]), 0) + 1
    return {"programs": m["total"], "interpreter_green": m["interp_green"], "silent_divergences_recorded": m["silent"],
            "silent_divergence_rate": round(m["silent"] / m["interp_green"], 3) if m["interp_green"] else None,
            "rechecked": len(m["rows"]), "compiled_error_classes": classes,
            "check_misses": sum(1 for r in m["rows"] if r["silent"] and r["check_accepts"])}
