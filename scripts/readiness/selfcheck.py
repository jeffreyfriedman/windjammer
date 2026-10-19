"""Cross-checks that a rubber-stamp `wj` (one that passes every test, prints fixed output, accepts every
program) cannot get past: canaries whose right answer is known independently of the binary under test."""
import json, os, tempfile

CANARY_TESTS = "fn test_canary_pass() {\n    assert(1 + 1 == 2)\n}\n\nfn test_canary_fail() {\n    assert(1 + 1 == 3)\n}\n"
CANARY_SEED = "fn main() {\n    let s: int = __SEED__\n    println(\"${s * 7 + 3}\")\n}\n"
CANARY_BAD = "fn main() {\n    let x: int = \"s\"\n    println(\"${x}\")\n}\n"


def run_canaries(ctx):
    issues = []
    d = tempfile.mkdtemp(prefix="wj-canary-")
    os.makedirs(os.path.join(d, "t"))
    with open(os.path.join(d, "t", "canary_test.wj"), "w") as f:
        f.write(CANARY_TESTS)
    r = ctx.runner([ctx.wj, "test", "--interpret", "--json", os.path.join(d, "t")], timeout=ctx.timeouts["interp"], cwd=d)
    try:
        doc = json.loads(r.out[r.out.find("{"):r.out.rindex("}") + 1])
        got = {t["name"]: t["passed"] for t in doc["tests"]}
    except (ValueError, KeyError):
        got = {}
    if got.get("test_canary_fail") is not False or got.get("test_canary_pass") is not True:
        issues.append(f"canary tests: expected pass=True/fail=False, saw {got or 'no test list'} (a binary that passes everything is not a measurement)")
    if r.rc == 0:
        issues.append("canary tests: `wj test --interpret` exited 0 although a test fails")
    outs = []
    for seed in (3, 11):
        p = os.path.join(d, f"seed{seed}.wj")
        open(p, "w").write(CANARY_SEED.replace("__SEED__", str(seed)))
        o = ctx.runner([ctx.wj, "run", "--interpret", "--no-lint", p], timeout=ctx.timeouts["interp"], cwd=d)
        outs.append((o.out.strip(), str(seed * 7 + 3)))
    if any(a != b for a, b in outs):
        issues.append(f"canary seeds: interpreter printed {[a for a, _ in outs]}, independent arithmetic says {[b for _, b in outs]}")
    bad = os.path.join(d, "bad.wj")
    open(bad, "w").write(CANARY_BAD)
    c = ctx.runner([ctx.wj, "check", bad], timeout=ctx.timeouts["check"], cwd=d)
    i = ctx.runner([ctx.wj, "run", "--interpret", bad], timeout=ctx.timeouts["interp"], cwd=d)
    if c.rc == 0:
        issues.append("canary type error: `wj check` accepted `let x: int = \"s\"`")
    if i.rc == 0:
        issues.append("canary type error: the interpreter ran `let x: int = \"s\"`")
    return issues


def cross_checks(criteria):
    """Consistency of the measurements themselves."""
    issues = []
    r1 = next((c for c in criteria if c["id"] == "R1"), None)
    if r1:
        for row in r1["detail"].get("projects", []):
            if row.get("measured") and row.get("sets_equal") is False:
                issues.append(f"R1 {row['project']}: interpreted and compiled runs discovered different test sets")
            if row.get("interp_only_pass"):
                issues.append(f"R1 {row['project']}: {len(row['interp_only_pass'])} test(s) pass interpreted but not compiled (interpreter more permissive)")
    return issues


def run_all(ctx, criteria):
    return {"issues": run_canaries(ctx) + cross_checks(criteria)}
