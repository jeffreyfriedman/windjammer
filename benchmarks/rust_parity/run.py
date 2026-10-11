#!/usr/bin/env python3
"""Windjammer-vs-idiomatic-Rust parity harness.

For every kernel K it builds three release (-C opt-level=3) binaries:
  baseline   generated_baseline/K.rs      (compiler output WITHOUT the optimizer; snapshot)
  optimized  optimize_rs(baseline)        (exactly what the compiler hook emits)
  idiomatic  rust/K.rs                    (hand-written Rust)
verifies all print identical output, then reports median/min/max/stdev of N
runs (default 11), binary size, source counts (.clone(), .to_string(), casts)
and heap allocation counts from a counting-allocator build.

usage: run.py --optimize-rs <path to built example> [--runs 11] [--out results.json]
Build the example first:  cargo build --release --example optimize_rs
"""
import argparse
import json
import os
import re
import resource
import statistics
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
KERNELS = ["sieve", "matmul", "collatz", "string_build", "fib", "vec_fill",
           "insertion_sort", "nbody", "byte_hash", "histogram"]

COUNTING_ALLOC = r'''
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
pub struct __Counting;
pub static __ALLOCS: AtomicUsize = AtomicUsize::new(0);
pub static __BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for __Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        __ALLOCS.fetch_add(1, Ordering::Relaxed);
        __BYTES.fetch_add(l.size(), Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        __ALLOCS.fetch_add(1, Ordering::Relaxed);
        __BYTES.fetch_add(n, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static __A: __Counting = __Counting;
fn main() {
    __wj_main();
    eprintln!("ALLOCS {} {}", __ALLOCS.load(Ordering::Relaxed), __BYTES.load(Ordering::Relaxed));
}
'''


def sh(cmd, **kw):
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw)


def build(src, out, counting=False):
    text = open(src).read()
    if counting:
        text = re.sub(r"\bfn main\(\)", "fn __wj_main()", text, count=1) + COUNTING_ALLOC
        src = out + ".rs"
        open(src, "w").write(text)
    sh(["rustc", "--edition", "2021", "-C", "opt-level=3", "-A", "warnings", "-o", out, src])


def counts(path):
    code = "\n".join(l for l in open(path).read().splitlines() if not l.strip().startswith("//"))
    return {"clone": code.count(".clone()"), "to_string": code.count(".to_string()"),
            "as_cast": len(code.split(" as ")) - 1, "format": code.count("format!("),
            "lines": len(code.splitlines())}


def time_runs(binary, n):
    walls, cpus, out = [], [], None
    for _ in range(n):
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        t0 = time.perf_counter()
        r = subprocess.run([binary], capture_output=True, text=True, check=True)
        walls.append(time.perf_counter() - t0)
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpus.append((after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime))
        out = r.stdout
    return walls, cpus, out


def summarize(xs):
    return {"median": statistics.median(xs), "min": min(xs), "max": max(xs),
            "stdev": statistics.pstdev(xs), "n": len(xs)}


def alloc_count(binary):
    r = subprocess.run([binary], capture_output=True, text=True, check=True)
    m = re.search(r"ALLOCS (\d+) (\d+)", r.stderr)
    return {"allocs": int(m.group(1)), "bytes": int(m.group(2))}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--optimize-rs", required=True)
    ap.add_argument("--runs", type=int, default=11)
    ap.add_argument("--out", default=os.path.join(HERE, "results.json"))
    ap.add_argument("--kernels", nargs="*", default=KERNELS)
    a = ap.parse_args()
    work = tempfile.mkdtemp(prefix="wj-parity-")
    results = {}
    for k in a.kernels:
        srcs = {"baseline": os.path.join(HERE, "generated_baseline", k + ".rs"),
                "idiomatic": os.path.join(HERE, "rust", k + ".rs")}
        opt = os.path.join(work, k + "_optimized.rs")
        sh([a.optimize_rs, srcs["baseline"], opt])
        srcs["optimized"] = opt
        row, outputs = {}, {}
        for v, src in srcs.items():
            binary = os.path.join(work, f"{k}_{v}")
            build(src, binary)
            walls, cpus, out = time_runs(binary, a.runs)
            cbin = binary + "_count"
            build(src, cbin, counting=True)
            row[v] = {"wall": summarize(walls), "cpu": summarize(cpus),
                      "binary_bytes": os.path.getsize(binary), "source": counts(src),
                      "heap": alloc_count(cbin)}
            outputs[v] = out
        assert len(set(outputs.values())) == 1, f"{k}: outputs differ {outputs}"
        row["output"] = outputs["baseline"].strip()
        results[k] = row
        print(k, {v: round(row[v]["wall"]["median"], 3) for v in srcs}, file=sys.stderr)
    json.dump({"runs": a.runs, "kernels": results}, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
