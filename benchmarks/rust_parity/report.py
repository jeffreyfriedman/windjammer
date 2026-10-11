#!/usr/bin/env python3
"""Render results.json (from run.py) as a markdown table on stdout."""
import json
import sys

d = json.load(open(sys.argv[1]))
print(f"Runs per binary: {d['runs']}. Times are seconds; `cpu` = user+sys of the child.\n")
print("| kernel | variant | wall median | wall min..max | wall stdev | cpu median | size KB | allocs | alloc MB | clone | to_string | as | format! |")
print("|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
for k, row in d["kernels"].items():
    for v in ("baseline", "optimized", "idiomatic"):
        r = row[v]
        w, c, s, h = r["wall"], r["cpu"], r["source"], r["heap"]
        print(f"| {k} | {v} | {w['median']:.3f} | {w['min']:.3f}..{w['max']:.3f} | {w['stdev']:.3f} | {c['median']:.3f} | "
              f"{r['binary_bytes']/1024:.0f} | {h['allocs']} | {h['bytes']/1e6:.1f} | {s['clone']} | {s['to_string']} | {s['as_cast']} | {s['format']} |")
