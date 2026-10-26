#!/usr/bin/env bash
# Interpreter vs other runtimes on the bench/interp kernels (ADR-0020).
#
#   scripts/interp_compare.sh WJ_BIN [--runs N] [--json OUT.json] [--only NAME]
#
# `--json bench/interp/compare.json` records the run the readiness scorecard reads (per kernel:
# CPU seconds per runtime and `ratios.resolved_over_{node,python3,walker}`).
#
# For every bench/interp/<k>.wj: `wj run --interpret` with the tree walker and the resolved engine,
# the Rust the Windjammer compiler generates for the same file (`wj build --no-cargo`, then
# `rustc -O`, or `cargo build --release --offline` when it needs the runtime crate), and the
# hand-written twins bench/interp/twins/<k>.py (python3) and <k>.mjs (node). Every program must
# print exactly what the walker prints (the oracle); a mismatch is reported instead of a time.
# Times are child CPU seconds (user+sys), median of N runs (default 3), with the 1-minute load
# average and tool versions recorded. Prints a Markdown table; --json also writes the raw numbers.
set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 - "$@" <<'PY'
import json, os, resource, shutil, statistics, subprocess, sys, tempfile

args = sys.argv[1:]
if not args or args[0].startswith("-"):
    sys.exit("usage: scripts/interp_compare.sh WJ_BIN [--runs N] [--json OUT.json] [--only NAME]")
wj = os.path.abspath(args[0])
def opt(name, default=None):
    return args[args.index(name) + 1] if name in args else default
runs = int(opt("--runs", "3"))
only = opt("--only")
bench = "bench/interp"
kernels = sorted(f[:-3] for f in os.listdir(bench) if f.endswith(".wj") and (only is None or f[:-3] == only))
work = tempfile.mkdtemp(prefix="wj-interp-compare-")

def cpu(cmd, cwd=None):
    """Median child CPU seconds over `runs`, and the stdout of the first run (None on failure)."""
    times, out = [], None
    for _ in range(runs):
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        p = subprocess.run(cmd, cwd=cwd, capture_output=True, stdin=subprocess.DEVNULL)
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        if p.returncode != 0:
            return None, p.stderr.decode(errors="replace")[-300:]
        times.append((after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime))
        out = out if out is not None else p.stdout.decode()
    return statistics.median(times), out

def rust_binary(k):
    d = os.path.join(work, "rs_" + k)
    p = subprocess.run([wj, "build", f"{bench}/{k}.wj", "--no-cargo", "--no-lint", "-q", "-o", d], capture_output=True)
    src = os.path.join(d, k + ".rs")
    if p.returncode != 0 or not os.path.exists(src):
        return None, "wj build: " + (p.stderr.decode(errors="replace").strip().splitlines() or ["no output"])[-1][:80]
    exe = os.path.join(d, k)
    if subprocess.run(["rustc", "-O", "--edition", "2021", src, "-o", exe], capture_output=True).returncode == 0:
        return [exe], None
    c = subprocess.run(["cargo", "build", "--release", "--offline", "-q"], cwd=d, capture_output=True,
                       env={**os.environ, "CARGO_TARGET_DIR": os.path.join(d, "target")})
    exe = os.path.join(d, "target", "release", k)
    if c.returncode == 0 and os.path.exists(exe):
        return [exe], None
    first = next((l for l in c.stderr.decode(errors="replace").splitlines() if l.startswith("error")), "cargo build failed")
    return None, "rust: " + first[:80]

def versions():
    v = {}
    for name, cmd in [("python3", ["python3", "--version"]), ("node", ["node", "--version"]), ("rustc", ["rustc", "--version"])]:
        try:
            v[name] = subprocess.run(cmd, capture_output=True, text=True).stdout.strip()
        except OSError:
            v[name] = "missing"
    return v

res = {"load_avg_1m": round(os.getloadavg()[0], 1), "runs": runs, "versions": versions(), "kernels": {}}
cols = ["walker", "resolved", "rust", "python3", "node"]
for k in kernels:
    row, notes = {}, []
    base_t, oracle = cpu([wj, "run", "--interpret", "--no-lint", "--engine", "walker", f"{bench}/{k}.wj"])
    row["walker"] = base_t
    cmds = {"resolved": [wj, "run", "--interpret", "--no-lint", "--engine", "resolved", f"{bench}/{k}.wj"]}
    exe, why = rust_binary(k)
    if exe:
        cmds["rust"] = exe
    else:
        notes.append(why)
    for lang, ext in [("python3", "py"), ("node", "mjs")]:
        twin = f"{bench}/twins/{k}.{ext}"
        if os.path.exists(twin) and shutil.which(lang):
            cmds[lang] = [lang, twin]
    for name, cmd in cmds.items():
        t, out = cpu(cmd)
        if t is None:
            notes.append(f"{name} failed")
        elif out != oracle:
            notes.append(f"{name} output differs")
            t = None
        row[name] = t
    res["kernels"][k] = {"cpu_s": row, "notes": notes}

def fmt(t):
    return "n/a" if t is None else f"{t:.4f}"
print(f"load average {res['load_avg_1m']}, median of {runs}, CPU seconds; {res['versions']}")
print("| kernel | " + " | ".join(cols) + " | walker÷resolved | resolved÷python3 | resolved÷node | notes |")
print("|" + "---|" * (len(cols) + 5))
for k, v in res["kernels"].items():
    r = v["cpu_s"]
    ratio = lambda a, b: f"{r[a] / r[b]:.2f}x" if r.get(a) and r.get(b) else "n/a"
    print(f"| {k} | " + " | ".join(fmt(r.get(c)) for c in cols) + f" | {ratio('walker', 'resolved')} | {ratio('resolved', 'python3')} | {ratio('resolved', 'node')} | {'; '.join(v['notes'])} |")
# ratios for the readiness scorecard (bench/interp/compare.json): resolved engine over each runtime
for k, v in res["kernels"].items():
    r = v["cpu_s"]
    v["ratios"] = {f"resolved_over_{o}": round(r["resolved"] / r[o], 2) for o in ("node", "python3", "walker") if r.get("resolved") and r.get(o)}
out = opt("--json")
if out:
    with open(out, "w") as fh:
        json.dump(res, fh, indent=1)
        fh.write("\n")
shutil.rmtree(work, ignore_errors=True)
PY
