"""Pilot 7 preflight: the launch checks that can be automated, in one command that exits non-zero on any red.

    python3 -m agentdx pilot7-preflight --expect-commit <sha> [--accepted friction/pilot7-accepted.txt]

Why Python: part of the Python harness (agentdx/*.py); it reuses the dry run, the config loader and the report's
NOISY_LOAD directly. Python fallback: harness is Python, no wj blocker."""
import os
import re
import subprocess
import sys

from . import pilot7 as P7, toolchains
from .pilot6_report import NOISY_LOAD

DEFAULT_ACCEPTED = os.path.join("friction", "pilot7-accepted.txt")
DEFAULT_OPEN_BUGS = os.path.join("friction", "open-bugs.md")
DEFAULT_MIN_MEM_GB = 4.0


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
    reds = []
    for name, problems in checks:
        say(f"{'RED  ' if problems else 'GREEN'} {name}" + "".join(f"\n      {p}" for p in problems))
        if problems:
            reds.append(name)
    say("PREFLIGHT GREEN" if not reds else "PREFLIGHT RED: " + " ".join(reds))
    return 1 if reds else 0
