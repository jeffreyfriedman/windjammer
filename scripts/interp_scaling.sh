#!/usr/bin/env bash
# Scaling curves: the resolved engine vs python3 and node on bench/interp/scaling (ADR-0020 rung 2).
#
#   scripts/interp_scaling.sh WJ_BIN [--runs N] [--json OUT.json] [--only KERNEL] [--labels 1x,10x,100x]
#                                    [--engine resolved] [--check] [--bless]
#
# Why Python (not a .wj script): a recorded gap, not a preference. This is the same harness shape as
# scripts/interp_compare.sh (child rusage CPU time, /proc load guard, JSON) and the interpreter has no
# child-process rusage; the programs it measures are the generated .wj/.mjs/.py kernels.
#
# Programs: bench/interp/scaling/gen/<label>/<kernel>.{wj,mjs,py} (regenerate with gen.wj; sizes in
# bench/interp/scaling/sizes.toml; `work` there is the multiplier against 1x). Each program prints the
# kernel's result on stdout (the oracle: bench/interp/scaling/expected/<label>/<kernel>.out, byte for byte)
# and `STEADY_NS <n>` on stderr: the in-process wall clock around the kernel's whole body (data set-up
# and the hot loops; not interpreter/engine start-up, parsing or import). The clock is time::now_unix_nano
# in the .wj programs, process.hrtime.bigint() in node and time.perf_counter_ns() in python3.
# Per kernel, label and runtime (median of --runs, default 3):
#   total_cpu_s    child CPU seconds (user+sys), process start to exit, as interp_compare.sh records
#   total_wall_s   wall seconds of the same runs (node's GC/JIT threads make its CPU exceed its wall)
#   steady_s       median STEADY_NS (wall): the same method on all three sides
# and the curves: ratio resolved/node and resolved/python3 for total_cpu_s and steady_s at every label,
# plus `trend` = ratio at the largest label / ratio at 1x (above 1.15: the engine loses ground as sizes grow).
# --check: one untimed run of every program, outputs against expected/, no guard, no times (exit 1 on a diff).
# --bless: write expected/ from node's output (then --check proves the other two agree).
# Quiet-machine guard before every timed run (load average below 16: WJ_BENCH_MAX_LOAD; memory available
# at least 20%: WJ_BENCH_MIN_FREE_PCT); with --json a breach aborts (exit 3, nothing written).
set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 - "$@" <<'PY'
import json, os, resource, statistics, subprocess, sys, time
try:
    import tomllib
except ImportError:
    sys.exit("interp_scaling: python 3.11+ (tomllib) needed")

args = sys.argv[1:]
if not args or args[0].startswith("-"):
    sys.exit("usage: scripts/interp_scaling.sh WJ_BIN [--runs N] [--json OUT.json] [--only KERNEL] [--labels 1x,10x,100x] [--engine resolved] [--check] [--bless]")
wj = os.path.abspath(args[0])
def opt(name, default=None):
    return args[args.index(name) + 1] if name in args else default
runs = int(opt("--runs", "3"))
only = opt("--only")
engine = opt("--engine", "resolved")
check, bless = "--check" in args, "--bless" in args
root = "bench/interp/scaling"
sizes = tomllib.load(open(f"{root}/sizes.toml", "rb"))
labels = opt("--labels", "1x,10x,100x").split(",")
kernels = [k for k in sizes if only is None or k == only]
MAX_LOAD = float(os.environ.get("WJ_BENCH_MAX_LOAD", "16"))
MIN_FREE = float(os.environ.get("WJ_BENCH_MIN_FREE_PCT", "20"))
peak = {"load": 0.0, "min_free_pct": 100.0}

def free_pct():
    try:
        m = dict(l.split(":", 1) for l in open("/proc/meminfo"))
        return 100.0 * int(m["MemAvailable"].split()[0]) / int(m["MemTotal"].split()[0])
    except Exception:
        return 100.0

def guard():
    load, free = os.getloadavg()[0], free_pct()
    peak["load"], peak["min_free_pct"] = max(peak["load"], load), min(peak["min_free_pct"], free)
    if opt("--json") and (load >= MAX_LOAD or free < MIN_FREE):
        sys.stderr.write(f"interp_scaling: machine not quiet: load {load:.1f} (limit < {MAX_LOAD:g}), memory available {free:.0f}% "
                         f"(limit >= {MIN_FREE:g}%); nothing recorded. Re-run when the machine is quiet.\n")
        sys.exit(3)

def command(rt, k, label):
    d = f"{root}/gen/{label}/{k}"
    return {"resolved": [wj, "run", "--interpret", "--no-lint", "--engine", engine, d + ".wj"],
            "python3": ["python3", d + ".py"], "node": ["node", d + ".mjs"]}[rt]

def one(cmd):
    """(cpu s, wall s, steady s or None, stdout) of one run; stdout None on failure."""
    guard() if not check and not bless else None
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    t = time.perf_counter()
    p = subprocess.run(cmd, capture_output=True, stdin=subprocess.DEVNULL)
    wall = time.perf_counter() - t
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if p.returncode != 0:
        return None, None, None, "exit %d: %s" % (p.returncode, p.stderr.decode(errors="replace")[-200:])
    steady = None
    for l in p.stderr.decode().splitlines():
        if l.startswith("STEADY_NS "):
            steady = int(l.split()[1]) / 1e9
    return (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime), wall, steady, p.stdout.decode()

def expected_path(k, label):
    return f"{root}/expected/{label}/{k}.out"

if bless:
    for k in kernels:
        for label in labels:
            _, _, _, out = one(command("node", k, label))
            os.makedirs(f"{root}/expected/{label}", exist_ok=True)
            open(expected_path(k, label), "w").write(out)
            print(f"blessed {label}/{k}: {out.strip()}")
    sys.exit(0)

bad = 0
res = {"runs": runs, "engine": engine, "load_avg_1m": round(os.getloadavg()[0], 1), "kernels": {}}
for k in kernels:
    kr = res["kernels"][k] = {}
    for label in labels:
        want = open(expected_path(k, label)).read()
        row = kr[label] = {"work": sizes[k][label]["work"]}
        for rt in ("resolved", "python3", "node"):
            cpus, walls, steadies, note = [], [], [], None
            for _ in range(1 if check else runs):
                c, w, s, out = one(command(rt, k, label))
                if c is None:
                    note = out
                    break
                if out != want:
                    note = f"output differs: {out.strip()[:60]!r} vs {want.strip()[:60]!r}"
                    break
                cpus.append(c); walls.append(w)
                if s is not None:
                    steadies.append(s)
            if note:
                bad += 1
                print(f"FAIL {label}/{k}/{rt}: {note}", file=sys.stderr)
                row[rt] = {"error": note}
            elif check:
                print(f"ok   {label}/{k}/{rt}", file=sys.stderr)
                row[rt] = {"ok": True, "has_steady": bool(steadies)}
                bad += 0 if steadies else 1
                if not steadies:
                    print(f"FAIL {label}/{k}/{rt}: no STEADY_NS on stderr", file=sys.stderr)
            else:
                row[rt] = {"total_cpu_s": statistics.median(cpus), "total_wall_s": statistics.median(walls),
                           "steady_s": statistics.median(steadies)}
if check:
    sys.exit(1 if bad else 0)

def ratio(a, b, key):
    try:
        return a[key] / b[key]
    except (KeyError, ZeroDivisionError, TypeError):
        return None
for k, kr in res["kernels"].items():
    for label, row in kr.items():
        for other in ("node", "python3"):
            row[f"resolved_over_{other}"] = {key: ratio(row["resolved"], row[other], key) for key in ("total_cpu_s", "steady_s")}
    first, last = kr[labels[0]], kr[labels[-1]]
    kr["trend"] = {f"resolved_over_{o}_{key}": (None if None in (last[f"resolved_over_{o}"][key], first[f"resolved_over_{o}"][key])
                    else last[f"resolved_over_{o}"][key] / first[f"resolved_over_{o}"][key])
                   for o in ("node", "python3") for key in ("total_cpu_s", "steady_s")}
res["peak_load_1m"], res["min_free_pct"] = round(peak["load"], 1), round(peak["min_free_pct"])

f = lambda x: "n/a" if x is None else f"{x:.4g}"
r = lambda x: "n/a" if x is None else f"{x:.2f}x"
print(f"engine {engine}, median of {runs}, load average {res['load_avg_1m']} (peak {res['peak_load_1m']})")
print("| kernel | size | work | resolved total | python3 total | node total | resolved steady | python3 steady | node steady | steady res/node | steady res/py | total res/node | total res/py |")
print("|" + "---|" * 13)
for k, kr in res["kernels"].items():
    for label in labels:
        row = kr[label]
        g = lambda rt, key: row[rt].get(key)
        print(f"| {k} | {label} | {row['work']} | " + " | ".join(f(g(rt, "total_cpu_s")) for rt in ("resolved", "python3", "node")) + " | "
              + " | ".join(f(g(rt, "steady_s")) for rt in ("resolved", "python3", "node")) + " | "
              + f"{r(row['resolved_over_node']['steady_s'])} | {r(row['resolved_over_python3']['steady_s'])} | "
              + f"{r(row['resolved_over_node']['total_cpu_s'])} | {r(row['resolved_over_python3']['total_cpu_s'])} |")
print("\n| kernel | steady trend vs node | steady trend vs python3 | total trend vs node | total trend vs python3 | worsens vs node |")
print("|---|---|---|---|---|---|")
for k, kr in res["kernels"].items():
    t = kr["trend"]
    worse = (t["resolved_over_node_steady_s"] or 0) > 1.15
    print(f"| {k} | {r(t['resolved_over_node_steady_s'])} | {r(t['resolved_over_python3_steady_s'])} | {r(t['resolved_over_node_total_cpu_s'])} | {r(t['resolved_over_python3_total_cpu_s'])} | {'YES' if worse else 'no'} |")
if peak["load"] >= MAX_LOAD or peak["min_free_pct"] < MIN_FREE:
    print(f"warning: the machine was not quiet (peak load {peak['load']:.1f}, lowest memory available {peak['min_free_pct']:.0f}%): these numbers are noisy")
if opt("--json"):
    with open(opt("--json"), "w") as fh:
        json.dump(res, fh, indent=1)
        fh.write("\n")
PY
