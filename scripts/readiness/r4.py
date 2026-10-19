"""R4: fast enough. Interpreted vs compiled test wall time per project (from the R1 runs), the warm
edit-run loop of the largest project, and the bench/interp kernels against python3 (generated equivalents)."""
import glob, os, shutil, statistics, tempfile
from .core import pct
from .pygen import translate, Untranslatable

LOOP_TARGET_S = 1.0
LOOP_FILES = 50


def project_rows(r1_rows):
    rows, blockers = [], []
    for r in r1_rows:
        if not r["measured"]:
            blockers.append({"program": r["project"], "reason": "unmeasured: compiled run unavailable"})
            rows.append(dict(project=r["project"], met=None))
            continue
        met = r["interp_secs"] <= r["compiled_secs"]
        rows.append(dict(project=r["project"], interp_s=r["interp_secs"], compiled_s=r["compiled_secs"], met=met))
        if not met:
            blockers.append({"program": r["project"], "reason": f"interpreted {r['interp_secs']}s slower than compiled {r['compiled_secs']}s"})
    return rows, blockers


def warm_loop(ctx):
    """Edit one source file and re-run the interpreted suite, on a scratch copy of the largest project."""
    cands = sorted(ctx.projects, key=lambda p: len(p.wj_files()), reverse=True)
    if not cands:
        return None
    proj = cands[0]
    n = 2 if ctx.quick else 4
    tmp = tempfile.mkdtemp(prefix="wj-loop-")
    try:
        dst = os.path.join(tmp, "p")
        shutil.copytree(proj.path, dst, ignore=shutil.ignore_patterns("build", "target", ".git"))
        files = [f for f in sorted(glob.glob(os.path.join(dst, "**", "*.wj"), recursive=True)) if not f.endswith("_test.wj")]
        if not files:
            return None
        times = []
        for k in range(n + 1):
            with open(files[-1], "a") as f:
                f.write(f"\n// edit {k}\n")
            r = ctx.runner([ctx.wj, "test", "--interpret", dst], timeout=ctx.timeouts["interp"], cwd=dst)
            if k:  # the first run warms disk caches
                times.append(r.secs)
        return {"project": proj.label, "files": len(proj.wj_files()), "median_s": round(statistics.median(times), 2)}
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def kernels(ctx):
    rows, blockers = [], []
    reps = 1 if ctx.quick else 3
    for f in sorted(glob.glob(os.path.join(ctx.root, "bench", "interp", "*.wj"))):
        name = os.path.basename(f)
        try:
            py = translate(open(f).read())
        except Untranslatable as e:
            rows.append(dict(kernel=name, met=None))
            blockers.append({"program": "bench/" + name, "reason": f"unmeasured: python equivalent not generated ({e})"})
            continue
        pyfile = os.path.join(tempfile.mkdtemp(prefix="wj-pyk-"), name[:-3] + ".py")
        open(pyfile, "w").write(py)
        iv, pv = [], []
        for _ in range(reps):
            i = ctx.runner([ctx.wj, "run", "--interpret", f], timeout=ctx.timeouts["interp"])
            p = ctx.runner(["python3", "-I", pyfile], timeout=ctx.timeouts["interp"])
            iv.append(i); pv.append(p)
        same = iv[0].out.strip() == pv[0].out.strip()
        ti, tp = statistics.median(x.cpu for x in iv), statistics.median(x.cpu for x in pv)
        met = same and ti <= tp
        rows.append(dict(kernel=name, interp_cpu_s=round(ti, 2), python_cpu_s=round(tp, 2), ratio=round(ti / tp, 1) if tp else None, outputs_match=same, met=met))
        if not same:
            blockers.append({"program": "bench/" + name, "reason": "interpreter and python outputs differ"})
        elif not met:
            blockers.append({"program": "bench/" + name, "reason": f"interpreter {ti:.2f}s vs python {tp:.2f}s CPU ({(ti / tp) if tp else float('inf'):.1f}x slower)"})
    return rows, blockers


def measure(ctx, r1_rows):
    prow, pblk = project_rows(r1_rows)
    loop = warm_loop(ctx)
    krows, kblk = kernels(ctx)
    blockers = pblk + kblk
    loop_met = None
    if loop:
        loop_met = loop["median_s"] <= LOOP_TARGET_S
        if not loop_met:
            blockers.append({"program": loop["project"], "reason": f"warm edit-run loop {loop['median_s']}s > {LOOP_TARGET_S}s ({loop['files']} files)"})
    checks = [r["met"] for r in prow] + [r["met"] for r in krows] + [loop_met]
    return summarize(prow, loop, krows, checks, blockers, ctx.cold)


def summarize(prow, loop, krows, checks, blockers, cold=False):
    known = [c for c in checks if c is not None]
    status = "unmeasured" if not known else ("pass" if all(known) and len(known) == len(checks) else ("fail" if not all(known) else "unmeasured"))
    return {"id": "R4", "title": "Fast enough", "target": "interpreted <= compiled build+test; warm loop <= 1s; kernels >= python",
            "percent": pct(sum(known), len(known)), "status": status,
            "detail": {"projects": prow, "warm_loop": loop, "kernels": krows, "compiled_cache": "cold (fresh target dir per project)" if cold else "warm (private target dir, reused across projects)"},
            "blockers": blockers}
