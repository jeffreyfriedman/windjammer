#!/usr/bin/env python3
"""Interpreter benchmark ratchet (ADR-0020).

Runs every bench/interp/*.wj under `wj run --interpret` with the walker and the resolved engine,
5 times each, and reports the median user CPU time (child rusage, so wall-clock noise from a
loaded machine does not count) together with the 1-minute load average.

  scripts/interp_bench.py WJ_BIN                 print the table (JSON on stdout)
  scripts/interp_bench.py WJ_BIN --check         fail if a speedup (walker/resolved) falls below
                                                 bench/interp/baseline.json minus 15%
  scripts/interp_bench.py WJ_BIN --write         record the measured speedups as the new baseline
Speedups are ratios on the same machine and run, so they are comparable across machines.
"""
import json, os, statistics, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BENCH = os.path.join(ROOT, "bench", "interp")
BASELINE = os.path.join(BENCH, "baseline.json")
RUNS = 5


def measure(wj, prog, engine):
    times, outs = [], set()
    for _ in range(RUNS):
        pid = os.fork()
        if pid == 0:
            devnull = os.open(os.devnull, os.O_WRONLY)
            os.dup2(devnull, 2)
            os.dup2(devnull, 1)
            os.execv(wj, [wj, "run", "--interpret", "--engine", engine, prog])
        _, status, ru = os.wait4(pid, 0)
        if os.waitstatus_to_exitcode(status) != 0:
            sys.exit(f"{prog} ({engine}) failed")
        times.append(ru.ru_utime)
    return statistics.median(times)


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    wj = os.path.abspath(sys.argv[1])
    progs = sorted(f for f in os.listdir(BENCH) if f.endswith(".wj"))
    res = {"load_avg_1m": round(os.getloadavg()[0], 1), "runs": RUNS, "programs": {}}
    for f in progs:
        p = os.path.join(BENCH, f)
        w = measure(wj, p, "walker")
        r = measure(wj, p, "resolved")
        res["programs"][f] = {"walker_s": round(w, 3), "resolved_s": round(r, 3), "speedup": round(w / r, 2) if r else None}
    print(json.dumps(res, indent=1))
    if "--write" in sys.argv:
        with open(BASELINE, "w") as fh:
            json.dump({k: v["speedup"] for k, v in res["programs"].items()}, fh, indent=1)
            fh.write("\n")
    if "--check" in sys.argv:
        base = json.load(open(BASELINE))
        bad = [f"{k}: {res['programs'][k]['speedup']} < {v} - 15%" for k, v in base.items()
               if k in res["programs"] and res["programs"][k]["speedup"] < v * 0.85]
        if bad:
            sys.exit("speedup regressed:\n" + "\n".join(bad))


if __name__ == "__main__":
    main()
