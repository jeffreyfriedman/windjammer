"""Toolchain detection. Missing toolchains are skipped LOUDLY, never silently."""
import os
import shutil
import subprocess
import sys


def _which(name, env=None):
    return shutil.which(name)


def wj_bin():
    return os.environ.get("WJ_BIN") or shutil.which("wj")


def detect():
    """Return {lang: {"available": bool, "reason": str, "version": str}}."""
    res = {}

    def ver(cmd):
        try:
            p = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
            return (p.stdout or p.stderr).strip().splitlines()[0]
        except Exception as e:  # noqa: BLE001
            return f"unknown ({e})"

    wj = wj_bin()
    if wj:
        v = ver([wj, "--version"])
        res["wj-interp"] = {"available": True, "version": v, "reason": ""}
        if shutil.which("cargo"):
            res["wj"] = {"available": True, "version": v + " + cargo", "reason": ""}
        else:
            res["wj"] = {"available": False, "version": v, "reason": "cargo not on PATH (compiled path needs it)"}
    else:
        for k in ("wj-interp", "wj"):
            res[k] = {"available": False, "version": "", "reason": "wj not found (set WJ_BIN)"}
    for lang, exe, vc in (("rust", "rustc", ["rustc", "--version"]),
                          ("go", "go", ["go", "version"]),
                          ("python", sys.executable, [sys.executable, "--version"])):
        if exe and shutil.which(exe) or (exe == sys.executable):
            res[lang] = {"available": True, "version": ver(vc), "reason": ""}
        else:
            res[lang] = {"available": False, "version": "", "reason": f"{exe} not on PATH"}
    node = shutil.which("node")
    if node:
        checker = "tsc" if shutil.which("tsc") else ("deno" if shutil.which("deno") else None)
        v = ver(["node", "--version"])
        if checker:
            res["ts"] = {"available": True, "version": f"node {v}; typecheck via {checker}", "reason": ""}
        else:
            res["ts"] = {"available": True, "version": f"node {v}; NO type checker (tsc/deno missing)",
                         "reason": "types are stripped, not checked"}
    else:
        res["ts"] = {"available": False, "version": "", "reason": "node not on PATH"}
    return res


def announce(tc, langs, out=sys.stderr):
    """Print loud SKIP lines; return the langs that are usable."""
    ok = []
    for l in langs:
        info = tc.get(l, {"available": False, "reason": "unknown language"})
        if info["available"]:
            ok.append(l)
            if info.get("reason"):
                print(f"NOTE  [{l}] {info['reason']}", file=out)
        else:
            print(f"SKIP  [{l}] toolchain missing: {info['reason']}", file=out)
    return ok
