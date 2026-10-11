#!/usr/bin/env python3
"""Measure the Windjammer interpreter against .wj programs.

Usage: interpreter_coverage.py WJ_BIN OUT_JSON [--corpus DIR] [--rust-diff [--rust-cache FILE]] [--strip-banner] [--driver TEST_BIN] [ROOT ...]

For every *.wj under ROOT (default: tests examples) that defines `fn main`:
  * interp_ok   : `wj run --interpret` exits 0 within the timeout
  * rust_ran    : (with --rust-diff) `wj build --no-cargo` + rustc compiled, and the binary ran
  * match       : rust_ran and interpreter stdout + exit code equal the compiled binary's
For --corpus DIR (programs with `// expect:` / `// seeds:` / `.vectors/<seed>.out`
oracles, the conventions used by the cross-backend conformance corpus):
  * oracle_ok   : interpreter stdout equals the independent oracle for every seed
Nothing here prints on behalf of the program: stdout is captured from real processes.
"""
import json, os, re, subprocess, sys, tempfile, shutil

TIMEOUT = 20


def run(cmd, **kw):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=TIMEOUT, **kw)
    except subprocess.TimeoutExpired:
        return None


STRIP_BANNER = False
DRIVER = None  # test binary exposing interpreter_probe_test::probe_one (in-process, faster in debug builds)


class Result:
    def __init__(self, stdout, returncode, stderr):
        self.stdout, self.returncode, self.stderr = stdout, returncode, stderr


def interp(wj, path):
    if DRIVER:
        env = dict(os.environ, WJ_PROBE_FILE=os.path.abspath(path))
        p = run([DRIVER, "interpreter_probe_test::probe_one", "--exact", "--ignored", "--nocapture"], env=env)
        if p is None:
            return None
        m = re.search(r"@@STDOUT-BEGIN\n(.*?)@@STDOUT-END\n@@EXIT (-?\d+)\n@@ERR (.*)", p.stdout, re.S)
        if not m:
            return Result("", p.returncode or 1, "driver crashed: " + p.stderr[-200:])
        return Result(m.group(1), int(m.group(2)), m.group(3).replace("\\n", "\n"))
    r = run([wj, "run", "--interpret", path])
    if r is not None and STRIP_BANNER:
        # pre-fix binaries print an "Interpreting ..." banner on stdout; drop it so the
        # baseline measures the evaluator, not the banner
        lines = r.stdout.splitlines(keepends=True)
        if lines and lines[0].startswith("Interpreting "):
            r.stdout = "".join(lines[1:])
    return r


def rust_run(wj, src_path):
    with tempfile.TemporaryDirectory() as d:
        shutil.copy(src_path, os.path.join(d, "test.wj"))
        r = run([wj, "build", "--target", "rust", "--no-cargo", "test.wj"], cwd=d)
        rs = os.path.join(d, "build", "test.rs")
        if r is None or r.returncode != 0 or not os.path.exists(rs):
            return None
        b = os.path.join(d, "bin")
        c = subprocess.run(["rustc", rs, "-o", b, "--edition", "2021"], capture_output=True, text=True)
        if c.returncode != 0:
            return None
        return run([b])


def header(text):
    seeds, expect, exit_ = [], [], 0
    for line in text.splitlines():
        if not line.startswith("// "):
            continue
        rest = line[3:]
        if rest.startswith("seeds:"):
            seeds = [int(s) for s in rest[6:].split() if s.isdigit()]
        elif rest.startswith("expect:"):
            v = rest[7:]
            expect.append(v[1:] if v.startswith(" ") else v)
        elif rest.startswith("exit:"):
            exit_ = int(rest[5:].strip() or 0)
    return seeds, expect, exit_


def corpus(wj, d):
    res = {}
    for f in sorted(os.listdir(d)):
        if not f.endswith(".wj"):
            continue
        name = f[:-3]
        text = open(os.path.join(d, f)).read()
        seeds, expect, exit_ = header(text)
        seeded = "__SEED__" in text
        if not seeds:
            seeds = [1, 2, 3, 4, 5] if seeded else [0]
        ok = True
        why = ""
        for s in seeds:
            with tempfile.TemporaryDirectory() as t:
                p = os.path.join(t, "p.wj")
                open(p, "w").write(text.replace("__SEED__", str(s)))
                r = interp(wj, p)
            vec = os.path.join(d, name + ".vectors", f"{s}.out")
            if os.path.exists(vec):
                want = open(vec).read()
            else:
                want = "".join(l + "\n" for l in expect)
            if r is None:
                ok, why = False, "timeout"
            elif r.stdout != want or r.returncode != exit_:
                ok, why = False, f"seed {s}: exit {r.returncode}; " + (r.stderr.strip().splitlines() or [""])[-1][:120]
            if not ok:
                break
        res[name] = {"oracle_ok": ok, "why": why}
    return res


def main():
    a = sys.argv[1:]
    wj, out = a[0], a[1]
    rest = a[2:]
    global STRIP_BANNER, DRIVER
    if "--driver" in a:
        i = a.index("--driver")
        DRIVER = a[i + 1]
        del a[i : i + 2]
        wj, out = a[0], a[1]
        rest = a[2:]
    rust_cache = None
    if "--rust-cache" in rest:
        i = rest.index("--rust-cache")
        rust_cache = rest[i + 1]
        del rest[i : i + 2]
    rust_diff = "--rust-diff" in rest
    STRIP_BANNER = "--strip-banner" in rest
    rest = [x for x in rest if x not in ("--rust-diff", "--strip-banner")]
    corp = None
    if "--corpus" in rest:
        i = rest.index("--corpus")
        corp = rest[i + 1]
        del rest[i : i + 2]
    roots = rest or ["tests", "examples"]
    files = []
    for r in roots:
        for dp, dn, fn in os.walk(r):
            dn[:] = [x for x in dn if x not in ("target", "build", "node_modules")]
            for f in fn:
                if f.endswith(".wj"):
                    p = os.path.join(dp, f)
                    if re.search(r"\bfn\s+main\s*\(", open(p, errors="replace").read()):
                        files.append(p)
    cache = {}
    if rust_cache and os.path.exists(rust_cache):
        cache = json.load(open(rust_cache))
    res = {"files": {}, "corpus": {}}
    for p in sorted(files):
        r = interp(wj, p)
        e = {"interp_ok": bool(r and r.returncode == 0)}
        if r is not None and r.returncode != 0:
            e["why"] = (r.stderr.strip().splitlines() or [""])[-1][:160]
        if r is None:
            e["why"] = "timeout"
        if rust_diff:
            if p not in cache:
                rr = rust_run(wj, p)
                cache[p] = {"ran": rr is not None, "stdout": rr.stdout if rr else "", "exit": rr.returncode if rr else -1}
                if rust_cache:
                    json.dump(cache, open(rust_cache, "w"))
            c = cache[p]
            e["rust_ran"] = c["ran"]
            e["match"] = bool(c["ran"] and r is not None and r.stdout == c["stdout"] and r.returncode == c["exit"])
        res["files"][p] = e
    if corp:
        res["corpus"] = corpus(wj, corp)
    n = len(res["files"])
    ok = sum(1 for e in res["files"].values() if e["interp_ok"])
    summ = {"files_with_main": n, "interp_ok": ok}
    if rust_diff:
        summ["rust_compiled_and_ran"] = sum(1 for e in res["files"].values() if e.get("rust_ran"))
        summ["match_rust_output_and_exit"] = sum(1 for e in res["files"].values() if e.get("match"))
    if corp:
        summ["corpus_total"] = len(res["corpus"])
        summ["corpus_oracle_ok"] = sum(1 for e in res["corpus"].values() if e["oracle_ok"])
    res["summary"] = summ
    json.dump(res, open(out, "w"), indent=1)
    print(json.dumps(summ))


if __name__ == "__main__":
    main()
