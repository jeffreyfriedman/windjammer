#!/usr/bin/env python3
"""Compiled-Windjammer arena benchmarks (ADR-0025).

Builds each program in wj/ with `wj build --release` semantics (generated crate, `cargo build --release`),
optionally with WJ_AUTO_ARENA=1, checks that variants print the same checksum, and reports the median and
min of N runs plus the 1/5/15-minute load average at measurement time.

usage: run_wj.py --wj <path to wj binary> [--runs 11]
"""
import argparse, os, statistics, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
CASES = [
    ("particles_vec", "particles_vec.wj", {}),
    ("particles_arena", "particles_arena.wj", {}),
    ("labels_loop", "labels_loop.wj", {}),
    ("labels_loop+auto_arena", "labels_loop.wj", {"WJ_AUTO_ARENA": "1"}),
]


def build(wj, src, out, env_extra, target):
    env = dict(os.environ, **env_extra)
    subprocess.run([wj, "build", "--no-cargo", "--no-lint", "-o", out, src], check=True, env=env,
                   stdout=subprocess.DEVNULL)
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=out, check=True,
                   env=dict(env, CARGO_TARGET_DIR=target), stderr=subprocess.DEVNULL)
    name = os.path.splitext(os.path.basename(src))[0]
    return os.path.join(target, "release", name)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--wj", required=True)
    ap.add_argument("--runs", type=int, default=11)
    a = ap.parse_args()
    tmp = tempfile.mkdtemp(prefix="wj-arena-bench-")
    target = os.path.join(tmp, "target")
    print("load before:", os.getloadavg())
    results = {}
    for label, file, env in CASES:
        out = os.path.join(tmp, label.replace("+", "_"))
        exe = build(a.wj, os.path.join(HERE, "wj", file), out, env, target + label.replace("+", "_"))
        gen = open(os.path.join(out, file.replace(".wj", ".rs"))).read()
        times, outputs = [], set()
        for _ in range(a.runs):
            t = time.perf_counter()
            r = subprocess.run([exe], capture_output=True, text=True, check=True)
            times.append(time.perf_counter() - t)
            outputs.add(r.stdout.strip())
        assert len(outputs) == 1, outputs
        results[label] = (statistics.median(times), min(times), outputs.pop(), ".clear();" in gen)
    print("load after:", os.getloadavg())
    for label, (med, mn, out, hoisted) in results.items():
        print(f"{label:<26} median {med*1e3:8.1f} ms  min {mn*1e3:8.1f} ms  output {out}  scratch_reuse applied: {hoisted}")
    pv, pa = results["particles_vec"][0], results["particles_arena"][0]
    lb, la = results["labels_loop"][0], results["labels_loop+auto_arena"][0]
    assert results["particles_vec"][2] == results["particles_arena"][2], "particle checksums differ"
    assert results["labels_loop"][2] == results["labels_loop+auto_arena"][2], "label checksums differ"
    print(f"particles: arena vs fresh Vec per frame: {pv/pa:.2f}x;  labels: auto-arena vs plain: {lb/la:.2f}x")


if __name__ == "__main__":
    sys.exit(main())
