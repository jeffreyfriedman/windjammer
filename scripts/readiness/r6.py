"""R6: stdlib and project coverage. std items with an interpreter builtin (std_interpreter ratchet), the
interpreter's documented unsupported features / capability refusals, and Rust-crate dependency coverage."""
import os, re
from .core import pct

_FN = re.compile(r"^(?:pub )?fn ([A-Za-z0-9_]+)\(", re.M)


def std_items(root):
    items = set()
    d = os.path.join(root, "std")
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if f.endswith(".wj") and f[:-3] not in ("basic_test", "test_simple"):
            items |= {f"{f[:-3]}::{n}" for n in _FN.findall(open(os.path.join(d, f)).read())}
    return items


def missing_items(root):
    p = os.path.join(root, "tests/data/std_interpreter_missing.txt")
    if not os.path.exists(p):
        return []
    return [l.strip() for l in open(p) if l.strip() and not l.startswith("#")]


def unsupported_features(root):
    p = os.path.join(root, "src/interpreter/features.rs")
    text = open(p).read() if os.path.exists(p) else ""
    return re.findall(r'^\s+no\("([^"]+)",\s*"[^"]*",\s*"([^"]*)"', text, re.M)


def uses_rust_crates(path):
    """A project depends on Rust crates when wj.toml lists non-path dependencies or it ships .rs sources outside build/."""
    toml = os.path.join(path, "wj.toml")
    if os.path.exists(toml):
        sect = None
        for line in open(toml):
            line = line.split("#")[0].strip()
            if line.startswith("["):
                sect = line
            elif sect == "[dependencies]" and "=" in line and "path" not in line:
                return True
    for d, dirs, files in os.walk(path):
        dirs[:] = [x for x in dirs if x not in ("build", "target", ".git")]
        if any(f.endswith(".rs") for f in files):
            return True
    return False


def measure(ctx, r1_rows):
    declared, missing = std_items(ctx.root), missing_items(ctx.root)
    unsup = unsupported_features(ctx.root)
    crate_projects = [p for p in ctx.projects if uses_rust_crates(p.path)]
    row = {r["project"]: r for r in r1_rows}
    uncovered = []
    for p in crate_projects:
        r = row.get(p.label)
        if not r or not r["measured"] or r["both_pass"] < r["compiled_pass"]:
            uncovered.append(p.label)
    blockers = [{"program": f"std::{m}", "reason": "no interpreter builtin and no capability refusal"} for m in missing]
    blockers += [{"program": f"feature:{i}", "reason": f"unsupported: {s[:90]}"} for i, s in unsup]
    blockers += [{"program": u, "reason": "Rust-crate dependency project not fully passing interpreted"} for u in uncovered]
    covered = len(declared) - len(missing)
    ok = not missing and not unsup and not uncovered and bool(declared)
    return {"id": "R6", "title": "Stdlib and project coverage", "target": "0 std items without builtin/refusal; crate deps run via host bridge",
            "percent": pct(covered, len(declared)), "status": "pass" if ok else ("unmeasured" if not declared else "fail"),
            "detail": {"std_declared": len(declared), "std_missing": len(missing), "unsupported_features": [i for i, _ in unsup],
                       "crate_dependency_projects": len(crate_projects), "crate_projects_uncovered": len(uncovered)},
            "blockers": blockers}
