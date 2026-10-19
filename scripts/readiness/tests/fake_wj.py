#!/usr/bin/env python3
"""A fake `wj` with scripted behavior for testing the scorecard (spec: JSON in $WJ_FAKE_SPEC).

spec = {"always_pass": bool,                       # rubber-stamp mode: every test passes, every program "agrees"
        "tests":   {"<project dir name>": {"interp": {name: bool}, "compiled": {name: bool}}},
        "run":     {"<file name>": {"interp": [stdout, rc], "compiled": [stdout, rc], "hash": {"1": stdout}}},
        "fallback": {"<file name>": "reason"},      # default `wj run` prints the fallback notice
        "check_reject": ["<file name>", ...]}       # `wj check` rejects these (and the honest interpreter refuses them)
Unscripted canary files behave like a correct implementation."""
import json, os, re, sys

spec = json.load(open(os.environ["WJ_FAKE_SPEC"]))
args = sys.argv[1:]
cmd = args[0] if args else ""
rest = args[1:]
path = next((a for a in reversed(rest) if not a.startswith("-") and not a.isdigit()), "")
base = os.path.basename(path.rstrip("/"))
always = spec.get("always_pass", False)


def emit(out="", rc=0, err=""):
    sys.stdout.write(out)
    sys.stderr.write(err)
    sys.exit(rc)


if cmd == "test":
    compiled = "--interpret" not in rest
    if base == "t":  # the selfcheck canary project
        tests = {"test_canary_pass": True, "test_canary_fail": True if always else False}
    else:
        t = spec["tests"].get(base, {"interp": {}, "compiled": {}})
        tests = t["compiled" if compiled else "interp"]
    if compiled:
        body = ",\n".join('    {"name": "%s", "file": "./tests/x_test.wj", "status": "%s"}' % (n, "passed" if ok else "failed") for n, ok in tests.items())
        emit('{\n  "command": "test",\n  "tests": [\n%s\n  ]\n}\n' % body, 0 if all(tests.values()) else 1)
    doc = {"mode": "interpreted", "tests": [{"file": "./tests/x_test.wj", "name": n, "passed": ok, "message": "" if ok else "boom"} for n, ok in tests.items()]}
    emit(json.dumps(doc), 0 if all(tests.values()) else 1)
elif cmd == "check":
    bad = base in spec.get("check_reject", []) or base == "bad.wj"
    emit("", 1 if (bad and not always) else 0, "error: type mismatch\n" if bad else "")
elif cmd == "build":
    out = rest[rest.index("-o") + 1] if "-o" in rest else "."
    open(os.path.join(out, os.path.splitext(base)[0] + ".rs"), "w").write("fn main() {}\n")
    emit()
elif cmd == "run":
    if base in spec.get("check_reject", []) or base == "bad.wj":
        if always or base in spec.get("permissive_run", []):
            emit("s\n")
        emit("", 2, "wj: not running: `wj check` reports 1 error\n")
    s = spec.get("run", {}).get(base)
    m = re.search(r"let s: int = (\d+)", open(path).read()) if os.path.exists(path) and s is None else None
    if m:
        seed = int(m.group(1))
        wrong = seed in spec.get("gen_wrong", [])
        emit("0\n" if always else f"{seed * 7 + 3 + (1 if wrong else 0)}\n")
    if "--compiled" in rest:
        v = (s or {}).get("compiled", ["", 0])
        emit(v[0], v[1], v[2] if len(v) > 2 else "")
    if "--interpret" in rest:
        hs = rest[rest.index("--hash-seed") + 1] if "--hash-seed" in rest else "0"
        if s is None:
            emit("", 0)
        out, rc = s["interp"]
        emit(s.get("hash", {}).get(hs, out), rc)
    if base in spec.get("fallback", {}):
        emit("", 0, "wj: running compiled (cargo): %s (a.wj:3:1); use `wj run --compiled` to skip this check\n" % spec["fallback"][base])
    out, rc = (s or {}).get("interp", ["", 0])
    emit(out, rc)
emit("", 0)
