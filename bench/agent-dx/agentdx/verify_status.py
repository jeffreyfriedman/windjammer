"""Verdict of `wj run --verify` for a reference solution, for the nine-references gate (preflight gate a).

Since the compiler prints `wj verify: OK, ... program exit status N` and exits 0 whenever the interpreted and compiled runs
agree, wj's own exit code no longer carries the program's status: the reported N is compared to the oracle's code instead.
Output with no such line (older compilers) falls back to the old rule: wj's exit code equals the oracle's code.
The shell gates (preflight.sh gate a, refs9.sh) implement the same rule; this is the tested statement of it.
Python fallback: harness is Python, no wj blocker."""
import re

OK_LINE = re.compile(r"wj verify: OK\b.*?program exit status (-?\d+)")
MATCH_PHRASE = "compiled run matches"


def reported_status(stderr):
    """-> the N of the `wj verify: OK ... program exit status N` line, or None when the output has no such line."""
    m = OK_LINE.search(stderr)
    return int(m.group(1)) if m else None


def reference_passes(rc, stderr, want_code):
    """True when the reference's verify run is a pass against the oracle exit code `want_code`."""
    if MATCH_PHRASE not in stderr:
        return False
    n = reported_status(stderr)
    if n is not None:
        return rc == 0 and n == int(want_code)
    return rc == int(want_code)
