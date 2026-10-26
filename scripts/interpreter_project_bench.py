#!/usr/bin/env python3
"""Measure `wj run` (interpreted) on generated projects; write the numbers to docs/generated/measured.json.

Usage: interpreter_project_bench.py WJ_BIN [--files 50] [--runs 15] [--write]

Measures (median wall-clock of RUNS runs, milliseconds):
  interpreter.run_hello_ms        `wj run hello.wj` (process start + load + run; per-run overhead)
  interpreter.project_50_cold_ms  `wj run` in a generated project with FILES modules (first run after writing it)
  interpreter.project_50_warm_ms  same project, median of later runs
  interpreter.watch_restart_ms    `wj run --watch`: time from a source edit to the new output on stdout
Each generated module has 3 functions and calls into another module; main calls every module.
"""
import json, os, statistics, subprocess, sys, tempfile, time


def gen(root, n):
    os.makedirs(os.path.join(root, "src", "pkg"), exist_ok=True)
    with open(os.path.join(root, "wj.toml"), "w") as f:
        f.write('[package]\nname = "bench"\nversion = "0.1.0"\n')
    for i in range(n):
        sub = "pkg/" if i % 2 else ""
        callee = (i - 1) if i > 0 else None
        with open(os.path.join(root, "src", f"{sub}m{i}.wj"), "w") as f:
            if callee is not None:
                cs = "pkg::" if callee % 2 else ""
                f.write(f"use crate::{cs}m{callee}\n\n")
            f.write(f"pub struct S{i} {{\n    pub v: int,\n}}\n\n")
            f.write(f"pub fn a(x: int) -> int {{\n    x * {i + 1} + 1\n}}\n\n")
            f.write(f"pub fn b(x: int) -> int {{\n    let s = S{i} {{ v: x }}\n    s.v + a(x)\n}}\n\n")
            body = f"m{callee}::a(x) + b(x)" if callee is not None else "b(x)"
            f.write(f"pub fn c(x: int) -> int {{\n    {body}\n}}\n")
    with open(os.path.join(root, "src", "pkg", "mod.wj"), "w") as f:
        for i in range(1, n, 2):
            f.write(f"pub mod m{i}\n")
    lines = []
    for i in range(n):
        p = f"pkg::m{i}" if i % 2 else f"m{i}"
        lines.append(f"    total = total + {p}::c({i})")
    with open(os.path.join(root, "src", "main.wj"), "w") as f:
        f.write("fn main() {\n    let mut total = 0\n" + "\n".join(lines) + '\n    println("${total}")\n}\n')


def timed(cmd, cwd):
    t = time.perf_counter()
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    dt = (time.perf_counter() - t) * 1000
    if p.returncode != 0:
        sys.exit(f"failed: {' '.join(cmd)}\n{p.stderr}")
    return dt, p.stdout


def watch_restart(wj, root):
    proc = subprocess.Popen([wj, "run", "--watch"], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    first = proc.stdout.readline()
    samples = []
    target = os.path.join(root, "src", "m0.wj")
    for k in range(5):
        time.sleep(0.3)
        t = time.perf_counter()
        with open(target, "a") as f:
            f.write(f"// edit {k}\n")
        line = proc.stdout.readline()
        samples.append((time.perf_counter() - t) * 1000)
    proc.kill()
    return statistics.median(samples), first


def main():
    args = sys.argv[1:]
    wj = os.path.abspath(args[0])
    n = int(args[args.index("--files") + 1]) if "--files" in args else 50
    runs = int(args[args.index("--runs") + 1]) if "--runs" in args else 15
    out = {}
    with tempfile.TemporaryDirectory() as d:
        with open(os.path.join(d, "hello.wj"), "w") as f:
            f.write('fn main() {\n    println("hi")\n}\n')
        out["interpreter.run_hello_ms"] = statistics.median(timed([wj, "run", "hello.wj"], d)[0] for _ in range(runs))
        proj = os.path.join(d, "proj")
        gen(proj, n)
        cold, _ = timed([wj, "run"], proj)
        out[f"interpreter.project_{n}_cold_ms"] = cold
        out[f"interpreter.project_{n}_warm_ms"] = statistics.median(timed([wj, "run"], proj)[0] for _ in range(runs))
        out["interpreter.watch_restart_ms"], _ = watch_restart(wj, proj)
    out = {k: round(v, 1) for k, v in out.items()}
    print(json.dumps(out, indent=1))
    if "--write" in args:
        path = os.path.join(os.path.dirname(__file__), "..", "docs", "generated", "measured.json")
        try:
            cur = json.load(open(path))
        except Exception:
            cur = {}
        for k, v in out.items():
            cur[k] = {"value": v, "source": "scripts/interpreter_project_bench.py"}
        with open(path, "w") as f:
            json.dump(cur, f, indent=2, sort_keys=True)
            f.write("\n")


if __name__ == "__main__":
    main()
