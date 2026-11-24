"""Pilot 7 preflight: the launch checks that can be automated, in one command that exits non-zero on any red.

    python3 -m agentdx pilot7-preflight --expect-commit <sha> [--accepted friction/pilot7-accepted.txt]

Why Python: part of the Python harness (agentdx/*.py); it reuses the dry run, the config loader and the report's
NOISY_LOAD directly. Python fallback: harness is Python, no wj blocker."""
import os
import re
import subprocess
import sys

import glob
import json
import shutil
import tempfile

from . import pilot7 as P7, toolchains
from .verify_status import reference_passes
from .pilot6_report import NOISY_LOAD

DEFAULT_ACCEPTED = os.path.join("friction", "pilot7-accepted.txt")
DEFAULT_OPEN_BUGS = os.path.join("friction", "open-bugs.md")
DEFAULT_MIN_MEM_GB = 4.0
BENCH_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # bench/agent-dx, wherever the repo is checked out
DEFAULT_TASKS = os.path.join(BENCH_ROOT, "tasks_set3")
DEFAULT_SOLUTIONS = os.path.join(BENCH_ROOT, "solutions", "wj-set3")
REFERENCE_GLOB = "t30[1-9]_*"
REFERENCE_TIMEOUT = 900


def check_wj(ident, expect_commit, environ=None):
    """-> list of red reasons: wj must be runnable, built from --expect-commit (prefix either way), clean and release,
    and the AGENTDX_* variables must not disagree with it."""
    red = []
    if not ident["version"]:
        return [f"`{ident['path']} --version` gave nothing (wj not found or not runnable)"]
    c = ident["commit"]
    if not c:
        red.append(f"no commit in the version line {ident['version']!r}")
    elif not (c.startswith(expect_commit) or expect_commit.startswith(c)):
        red.append(f"wj commit {c} is not the expected {expect_commit}")
    if not ident["clean"]:
        red.append("wj build is not 'clean'")
    if not ident["release"]:
        red.append("wj build is not 'release'")
    return red + toolchains.env_disagreements(ident, environ)


def check_load(load1, limit=NOISY_LOAD):
    return [f"load average {load1:.1f} > {limit:.1f} (NOISY_LOAD)"] if load1 > limit else []


def available_memory_gb():
    """MemAvailable from /proc/meminfo, else free+inactive+speculative pages from vm_stat (macOS); None if unknown."""
    try:
        for ln in open("/proc/meminfo"):
            if ln.startswith("MemAvailable:"):
                return int(ln.split()[1]) / 1048576.0
    except OSError:
        pass
    try:
        out = subprocess.run(["vm_stat"], capture_output=True, text=True, timeout=10).stdout
        size = int(re.search(r"page size of (\d+)", out).group(1))
        pages = sum(int(m.group(1)) for k in ("free", "inactive", "speculative") for m in [re.search(rf"Pages {k}:\s+(\d+)", out)] if m)
        return pages * size / 2 ** 30
    except Exception:  # noqa: BLE001
        return None


def check_memory(avail_gb, min_gb=DEFAULT_MIN_MEM_GB):
    if avail_gb is None:
        return ["available memory cannot be measured on this machine (pass --min-free-mem-gb 0 to waive)"] if min_gb > 0 else []
    return [f"available memory {avail_gb:.1f} GB < {min_gb:.1f} GB"] if avail_gb < min_gb else []


def open_event_ids(path):
    """Event ids in the table rows of open-bugs.md (the `events_open` list)."""
    if not os.path.exists(path):
        return None
    return [m.group(1) for ln in open(path) for m in [re.match(r"^\|\s*`([^`]+)`", ln)] if m]


def read_accepted(path):
    """-> ([(prefix, reason)], [malformed lines]). '#' comments and blank lines are ignored; a missing file is empty."""
    ok, bad = [], []
    if not os.path.exists(path):
        return ok, bad
    for n, ln in enumerate(open(path), 1):
        ln = ln.strip()
        if not ln or ln.startswith("#"):
            continue
        parts = ln.split(None, 1)
        (ok if len(parts) == 2 else bad).append(tuple(parts) if len(parts) == 2 else f"{path}:{n}: '{ln}' (format: <event-id-prefix> <reason>)")
    return ok, bad


def check_events(cfg, open_ids, accepted):
    """Open compiler-bug events on a configured task must each be covered by an accepted prefix with a reason. Event ids
    look like p6par-t104-V2-1-c00; the task is the `tNNN` part. Events on tasks outside the matrix do not matter."""
    if open_ids is None:
        return ["open-bugs.md not found"]
    mine = {t.split("_")[0] for t in cfg["tasks"]}
    red = []
    for e in open_ids:
        if not (set(e.split("-")) & mine):
            continue
        if not any(e.startswith(p) for p, _ in accepted):
            red.append(f"open event {e} is on a pilot-7 task and not accepted in pilot7-accepted.txt")
    return red


def default_wj(environ=None):
    """The wj binary for the references gate: --wj, else AGENTDX_WJ, WJ_BIN, else `wj` on PATH; None when none exists."""
    env = os.environ if environ is None else environ
    for cand in (env.get("AGENTDX_WJ"), env.get("WJ_BIN")):
        if cand:
            return cand
    return shutil.which("wj")


def run_wj_verify(wj, workdir, stdin_text, timeout=REFERENCE_TIMEOUT):
    """The real runner: `wj run --verify main.wj` in workdir with stdin_text -> (rc, stdout, stderr)."""
    try:
        r = subprocess.run([wj, "run", "--verify", "main.wj"], cwd=workdir, input=stdin_text, capture_output=True,
                           text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, "", f"timeout after {timeout} s"
    return r.returncode, r.stdout, r.stderr


def richest_vector(vectors_path):
    """The oracle vector with the longest stdin -> (stdin, expected stdout, expected exit code)."""
    v = max(json.load(open(vectors_path)), key=lambda y: len(y["stdin"]))
    return v["stdin"], v["stdout"], int(v.get("code", 0))


def check_references(wj, tasks_dir=DEFAULT_TASKS, solutions_dir=DEFAULT_SOLUTIONS, oracle_dir=None,
                     runner=run_wj_verify, say=lambda *_: None, pattern=REFERENCE_GLOB, expected=9):
    """Gate (a): every t301-t309 reference solution, run under `wj run --verify` on the oracle's richest vector, must give
    the oracle's stdout, the oracle's exit status (verify_status.reference_passes) and say the compiled run matches.
    The oracle is <oracle_dir>/<task>/oracle/vectors.json (default: inside the task directory). -> list of red reasons;
    a missing path is a red reason, never an exception. `runner(wj, workdir, stdin)` is injectable for tests."""
    if runner is run_wj_verify and not (wj and os.path.isfile(wj) and os.access(wj, os.X_OK)):
        return [f"wj binary {wj!r} is missing or not executable (--wj, AGENTDX_WJ, WJ_BIN or PATH)"]
    if not os.path.isdir(tasks_dir):
        return [f"references directory {tasks_dir} does not exist (--tasks-dir)"]
    if not os.path.isdir(solutions_dir):
        return [f"solutions directory {solutions_dir} does not exist (--solutions-dir)"]
    tasks = sorted(glob.glob(os.path.join(tasks_dir, pattern)))
    if len(tasks) != expected:
        return [f"found {len(tasks)} reference tasks matching {pattern} in {tasks_dir}, expected {expected}"]
    red = []
    tmp = tempfile.mkdtemp(prefix="p7refs-")
    try:
        for t in tasks:
            name = os.path.basename(t)
            src = os.path.join(solutions_dir, name, "main.wj")
            vec = os.path.join(oracle_dir, name, "oracle", "vectors.json") if oracle_dir else os.path.join(t, "oracle", "vectors.json")
            if not os.path.isfile(src):
                red.append(f"{name}: reference solution {src} is missing")
                continue
            if not os.path.isfile(vec):
                red.append(f"{name}: oracle {vec} is missing")
                continue
            stdin, want_out, want_code = richest_vector(vec)
            ws = os.path.join(tmp, name)
            os.makedirs(ws)
            shutil.copy(src, os.path.join(ws, "main.wj"))
            rc, out, err = runner(wj, ws, stdin)
            if reference_passes(rc, err, want_code) and out == want_out:
                say(f"        pass {name}")
            else:
                why = "stdout differs from the oracle" if out != want_out else "exit status or compiled-run check failed"
                first = next((l for l in err.splitlines() if l.startswith(("error", "warning"))), "")[:110]
                red.append(f"{name} (exit {rc}): {why} {first}".rstrip())
                say(f"        FAIL {red[-1]}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return red


def run_references(args, runner=run_wj_verify, say=print):
    """CLI of gate (a); same GREEN/RED lines and exit code as run(): -> 0 only when all nine pass."""
    wj = args.wj or default_wj()
    problems = check_references(wj, args.tasks_dir, args.solutions_dir, args.oracle_dir, runner, say)
    say(f"{'RED  ' if problems else 'GREEN'} references" + "".join(f"\n      {p}" for p in problems))
    say("PREFLIGHT GREEN" if not problems else "PREFLIGHT RED: references")
    return 1 if problems else 0


def run(args, environ=None, load1=None, mem_gb=None, say=print):
    """Run every check; print GREEN/RED lines; -> exit code (0 only when all green)."""
    cfg = P7.load_config(args.config)
    ident = toolchains.wj_identity()
    accepted, bad = read_accepted(args.accepted)
    prior = args.prior or ["results/pilot6/trials", "results/pilot7-validation/trials"]
    dry = P7.dry_run(cfg, prior, say=lambda *_: None)
    checks = [("wj", check_wj(ident, args.expect_commit, environ)),
              ("load", check_load(os.getloadavg()[0] if load1 is None else load1)),
              ("memory", check_memory(available_memory_gb() if mem_gb is None else mem_gb, args.min_free_mem_gb)),
              ("events", check_events(cfg, open_event_ids(args.open_bugs), accepted) + bad),
              ("dry-run", dry)]
    if getattr(args, "permission_probe", False):
        from . import permprobe
        checks.append(("permissions", permprobe.run_problems(args)))
    reds = []
    for name, problems in checks:
        say(f"{'RED  ' if problems else 'GREEN'} {name}" + "".join(f"\n      {p}" for p in problems))
        if problems:
            reds.append(name)
    say("PREFLIGHT GREEN" if not reds else "PREFLIGHT RED: " + " ".join(reds))
    return 1 if reds else 0
