"""R2: no silent divergence. Interpreted output vs (a) the program's own oracle, (b) the compiled run,
(c) itself under hash seeds 0..3; generated seeded programs vs their independent python reference model."""
import math, os, re
from .core import run, sha, pct, file_hash
from . import pilot

HASH_SEEDS_FULL, HASH_SEEDS_QUICK = (0, 1, 2, 3), (0, 1)
GEN_SEEDS_FULL = 300


def panic_message(err):
    """The panic text of either engine ('panic: msg' or rust's 'thread ... panicked at ...:\\nmsg')."""
    lines = err.splitlines()
    for i, l in enumerate(lines):
        if "panicked at" in l and i + 1 < len(lines):
            return lines[i + 1].strip()
        m = re.search(r"\bpanic: (.*)$", l)
        if m:
            return m.group(1).strip()
    return ""


def first_diff(a, b):
    al, bl = a.splitlines(), b.splitlines()
    for i in range(max(len(al), len(bl))):
        x = al[i] if i < len(al) else "<end>"
        y = bl[i] if i < len(bl) else "<end>"
        if x != y:
            return f"line {i + 1}: interpreted {x[:50]!r} vs expected {y[:50]!r}"
    return "output differs"


def interp(ctx, path, src_hash, hash_seed=0):
    return ctx.cached_run(f"r2-interp-h{hash_seed}", src_hash,
                          [ctx.wj, "run", "--interpret", "--no-lint", "--hash-seed", str(hash_seed), path],
                          ctx.timeouts["interp"], cwd=os.path.dirname(path))


def compiled(ctx, path, src_hash):
    path = ctx.isolated(path)
    return ctx.cached_run("r2-compiled-tmp", src_hash, [ctx.wj, "run", "--compiled", path], ctx.timeouts["compiled"],
                          cwd=os.path.dirname(path), env={"WJ_CARGO_TARGET_ROOT": ctx.target_root()})


def build_failed(c):
    return c.rc != 0 and ("could not compile" in c.err or "Cargo build failed" in c.err or "error[E" in c.err)


def build_error(c):
    return next((l.strip() for l in c.err.splitlines() if l.startswith("error")), "compiled build failed")[:110]


def judge_pair(name, i, c):
    """Divergence reasons between one interpreted and one compiled result (pure)."""
    if c.timed_out or i.timed_out:
        return None  # environmental, reported as unmeasured by the caller
    out = []
    if i.out != c.out:
        out.append(("stdout", first_diff(i.out, c.out)))
    if i.rc != c.rc:
        out.append(("exit", f"exit {i.rc} interpreted vs {c.rc} compiled"))
    pi, pc = panic_message(i.err), panic_message(c.err)
    if pi != pc and (pi or pc):
        out.append(("stderr", f"panic message {pi[:50]!r} vs {pc[:50]!r}"))
    return out


def check_program(ctx, p, hash_seeds, compile_it, results):
    """Fold one corpus program's checks into `results` (a dict with divergences/checked/unmeasured)."""
    h = file_hash(p.path)
    base = interp(ctx, p.path, h, 0)
    results["checked"] += 1
    want = p.expected_stdout()
    if want is not None and base.out != want:
        results["divergences"].append({"program": p.name, "kind": "oracle", "reason": first_diff(base.out, want)})
    if p.mode == "run" and base.rc != (p.exit or 0):
        results["divergences"].append({"program": p.name, "kind": "oracle", "reason": f"exit {base.rc}, expected {p.exit or 0}"})
    for hs in hash_seeds:
        if hs == 0:
            continue
        r = interp(ctx, p.path, h, hs)
        if (r.out, r.rc) != (base.out, base.rc):
            results["divergences"].append({"program": p.name, "kind": f"hash-seed {hs}", "reason": first_diff(r.out, base.out) if r.out != base.out else f"exit {r.rc} vs {base.rc}"})
    if compile_it:
        c = compiled(ctx, p.path, h)
        if c.timed_out:
            results["unmeasured"].append({"program": p.name, "reason": "compiled run timed out or was skipped (budget/disk)"})
        elif build_failed(c):
            # the Rust did not build: nothing ran, so there is no output to diverge; R5 owns this (interpreter accepts, compiled rejects)
            results["compiled_build_failures"] += 1
            results["build_failures"].append({"program": p.name, "reason": build_error(c)})
        else:
            results["compiled_checked"] += 1
            for kind, why in judge_pair(p.name, base, c) or []:
                results["divergences"].append({"program": p.name, "kind": "compiled-" + kind, "reason": why})


def reference(ctx, ref, seed):
    r = ctx.cached_run("r2-ref", sha(file_hash(ref), seed), ["python3", "-I", ref, str(seed)], 30)
    return r.out if r.rc == 0 else None


def gen_seeds(p, n_templates, quick):
    want = 1 if quick else max(math.ceil(GEN_SEEDS_FULL / max(n_templates, 1)), 1)
    seeds = list(p.seeds)[:want]
    s = 101
    while len(seeds) < want:
        seeds.append(s)
        s += 7
    return seeds


def check_generated(ctx, templates, hash_seeds, results):
    gdir = os.path.join(ctx.cache.dir, "gen")
    os.makedirs(gdir, exist_ok=True)
    for p in templates:
        for seed in gen_seeds(p, len(templates), ctx.quick):
            src = p.src.replace("__SEED__", str(seed))
            sh = sha(src)
            path = os.path.join(gdir, f"{p.name.split('/')[-1][:-3]}_{seed}.wj")
            with open(path, "w") as f:
                f.write(src)
            name = f"generated/{os.path.basename(p.path)}@{seed}"
            base = interp(ctx, path, sh, 0)
            results["checked"] += 1
            results["generated_seeds"] += 1
            want = reference(ctx, p.ref, seed)
            if want is None:
                results["unmeasured"].append({"program": name, "reason": "python reference model failed"})
            elif base.out != want:
                results["divergences"].append({"program": name, "kind": "oracle", "reason": first_diff(base.out, want)})
            if base.rc != (p.exit or 0):
                results["divergences"].append({"program": name, "kind": "oracle", "reason": f"exit {base.rc}, expected {p.exit or 0}"})
            for hs in hash_seeds:
                if hs and (interp(ctx, path, sh, hs).out != base.out):
                    results["divergences"].append({"program": name, "kind": f"hash-seed {hs}", "reason": "output depends on hash order"})


def measure(ctx, corpora, templates):
    hs = HASH_SEEDS_QUICK if ctx.quick else HASH_SEEDS_FULL
    limit = 1 if ctx.quick else 10**9
    res = {"divergences": [], "unmeasured": [], "checked": 0, "compiled_checked": 0, "generated_seeds": 0,
           "compiled_build_failures": 0, "build_failures": []}
    for name, progs in corpora.items():
        for k, p in enumerate(progs):
            check_program(ctx, p, hs, k < limit and not p.interp_only, res)
    check_generated(ctx, templates, hs, res)
    pm = pilot.measure(ctx)
    for r in pm["rows"]:
        res["checked"] += 1
        if r["silent"]:
            res["divergences"].append({"program": r["program"], "kind": "agent-pilot", "reason": f"interpreter-green, compiled fails: {r['rustc_error'][:100] or 'wrong output'}"})
    out = summarize(res, ctx.quick)
    out["detail"]["agent_pilot"] = pilot.summary(ctx)
    return out


def summarize(res, quick):
    div = res["divergences"]
    progs = {d["program"] for d in div}
    ok = res["checked"] - len(progs)
    seeds_ok = res["generated_seeds"] >= (1 if quick else GEN_SEEDS_FULL)
    status = "fail" if (div or res["build_failures"]) else ("pass" if res["checked"] and seeds_ok and not res["unmeasured"] else "unmeasured")
    return {"id": "R2", "title": "No silent divergence", "target": "0 divergent programs",
            "percent": pct(ok, res["checked"]), "status": status,
            "detail": {k: res[k] if not isinstance(res[k], list) else len(res[k]) for k in res} | {"hash_seeds": list(HASH_SEEDS_QUICK if quick else HASH_SEEDS_FULL)},
            "blockers": [{"program": d["program"], "reason": f"[{d['kind']}] {d['reason']}"} for d in div]
                        + [{"program": b["program"], "reason": "[compiled-build-failed] " + b["reason"]} for b in res["build_failures"]]
                        + [{"program": u["program"], "reason": "unmeasured: " + u["reason"]} for u in res["unmeasured"]]}
