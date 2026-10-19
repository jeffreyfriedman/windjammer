#!/usr/bin/env python3
"""Overhead of the capability gate (ADR-0021 gates: interpreter overhead <= 5% on general programs).

  scripts/bench_effects_overhead.py [--wj PATH] [--runs N] [--max-load L] [--json OUT]

Arms are interleaved (off, on, on, off, ...) and measured in child CPU time (user + sys), median of N runs; both
arms must print identical output. The run REFUSES to produce a verdict when the 1-minute load average exceeds
--max-load (default 20): a number measured on a loaded machine is not a result (see the benchmark rules in
docs/testing/). `load_avg_1m` is recorded next to every number.
"""
import argparse, json, os, resource, statistics, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKLOADS = {
    "non-sink (HashMap/Rc builtins, 400k iterations)": "bench/effects/nonsink.wj",
    "sink-dense (fs::exists + random + time, 30k iterations)": "bench/effects/sink.wj",
}


def run(wj, prog, caps):
    env = dict(os.environ, WJ_CAPS=caps)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    out = subprocess.run([wj, "run", "--interpret", "--deterministic-time", prog], env=env, capture_output=True, text=True, cwd=ROOT)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if out.returncode != 0:
        sys.exit(f"{prog} failed under WJ_CAPS={caps}: {out.stderr}")
    return (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime), out.stdout.strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--wj", default=os.environ.get("WJ", "wj"))
    ap.add_argument("--runs", type=int, default=9)
    ap.add_argument("--max-load", type=float, default=20.0)
    ap.add_argument("--json")
    a = ap.parse_args()
    load = os.getloadavg()[0]
    results = []
    for name, prog in WORKLOADS.items():
        run(a.wj, prog, "off"), run(a.wj, prog, "enforce")  # warm
        t = {"off": [], "enforce": []}
        outs = set()
        for _ in range(a.runs):
            for arm in ("off", "enforce", "enforce", "off"):
                c, o = run(a.wj, prog, arm)
                t[arm].append(c)
                outs.add(o)
        if len(outs) != 1:
            sys.exit(f"{name}: the arms printed different output: {outs}")
        m = {k: statistics.median(v) for k, v in t.items()}
        results.append({"workload": name, "off_s": m["off"], "on_s": m["enforce"], "overhead_pct": 100 * (m["enforce"] / m["off"] - 1), "runs_per_arm": 2 * a.runs})
    load_after = os.getloadavg()[0]
    valid = max(load, load_after) <= a.max_load
    report = {"load_avg_1m_before": load, "load_avg_1m_after": load_after, "max_load": a.max_load, "valid": valid, "cpu_time": True, "results": results}
    for r in results:
        print(f"{r['workload']}: off {r['off_s']:.3f}s on {r['on_s']:.3f}s overhead {r['overhead_pct']:+.1f}% ({r['runs_per_arm']} runs per arm)")
    print(f"load average {load:.1f} -> {load_after:.1f}: " + ("valid measurement" if valid else f"PROVISIONAL (load above {a.max_load}); re-measure on a quiet machine"))
    if a.json:
        json.dump(report, open(a.json, "w"), indent=1)
    sys.exit(0 if valid else 3)


if __name__ == "__main__":
    main()
