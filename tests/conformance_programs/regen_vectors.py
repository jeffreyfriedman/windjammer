"""Regenerate committed expected-output vectors from the independent reference models.

Usage: python3 regen_vectors.py   (run from anywhere; writes <name>.vectors/<seed>.out)
The Rust harness re-derives every vector with python3 when available and fails on drift.
"""
import pathlib, re, subprocess, sys

here = pathlib.Path(__file__).resolve().parent
for ref in sorted(list(here.glob("*.ref.py")) + list(here.glob("selftest/*.ref.py"))):
    name = ref.name[: -len(".ref.py")]
    wj = (ref.parent / f"{name}.wj").read_text()
    m = re.search(r"^// seeds:(.*)$", wj, re.M)
    seeds = [int(x) for x in m.group(1).split()] if m else [1, 2, 3, 4, 5]
    out_dir = ref.parent / f"{name}.vectors"
    out_dir.mkdir(exist_ok=True)
    for seed in seeds:
        res = subprocess.run([sys.executable, "-I", str(ref), str(seed)], capture_output=True, text=True, check=True)
        (out_dir / f"{seed}.out").write_text(res.stdout)
print("ok")
