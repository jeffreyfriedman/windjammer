"""`wj` shim placed first on the agent's PATH: forwards to the real compiler and logs every invocation
(verb, mode, exit code, seconds, output tail, source snapshot) outside the sandbox."""
import json
import os
import stat

SHIM = '''#!/usr/bin/env python3
import json, os, subprocess, sys, time
REAL, LOG, SRC = %(real)r, %(log)r, %(src)r
t0 = time.time()
p = subprocess.run([REAL] + sys.argv[1:], capture_output=True, text=True)
sys.stdout.write(p.stdout); sys.stderr.write(p.stderr)
try:
    src = open(SRC).read()[:20000]
except OSError:
    src = ""
with open(LOG, "a") as f:
    f.write(json.dumps({"argv": sys.argv[1:], "exit": p.returncode, "secs": round(time.time() - t0, 3),
                        "out": (p.stderr + p.stdout)[-2500:], "source": src}) + "\\n")
sys.exit(p.returncode)
'''


def install(bin_dir, real_wj, log_path, src_path, name="wj"):
    os.makedirs(bin_dir, exist_ok=True)
    p = os.path.join(bin_dir, name)
    with open(p, "w") as f:
        f.write(SHIM % {"real": real_wj, "log": log_path, "src": src_path})
    os.chmod(p, os.stat(p).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return p


def classify(argv, tool="wj"):
    """-> (verb, mode). mode in interpret|compiled|default|n/a; native toolchains are verb/mode "native"."""
    if tool != "wj":
        return "native", "native"
    args = [a for a in argv if not a.startswith("-")]
    verb = args[0] if args else ""
    if "--interpret" in argv:
        mode = "interpret"
    elif verb == "build" or "--compiled" in argv:
        mode = "compiled"
    elif verb in ("run", "test"):
        mode = "default"
    else:
        mode = "n/a"
    if verb == "check":
        mode = "interpret"   # check never runs cargo
    return verb, mode


VERIFY_VERBS = ("check", "run", "test", "build", "native")


def read_log(path, tool="wj"):
    if not os.path.exists(path):
        return []
    out = []
    for ln in open(path):
        if ln.strip():
            e = json.loads(ln)
            e["verb"], e["mode"] = classify(e["argv"], tool)
            out.append(e)
    return out
