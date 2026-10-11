import re
from collections import Counter


def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["2024-03-05T14:07:09 INFO web started", "2024-03-05T14:07:10 ERROR db connection lost", "garbage"]), _c([])]
EDGE = [_c(["", "2024-01-01T00:00:00 DEBUG a", "2024-01-01T00:00:00 WARN a x y z", "2023-12-31T23:59:59 ERROR b ", "2024-1-1T00:00:00 INFO a hi", "2024-01-01T00:00:00 TRACE a hi", "2024-01-01T00:00:00 INFO"]),
        _c([f"2024-01-0{1 + i % 9}T00:00:0{i % 10} INFO c{i % 5} m" for i in range(30)])]


def rand(rng):
    ls = []
    for _ in range(rng.randint(0, 25)):
        r = rng.random()
        if r < 0.15:
            ls.append(rng.choice(["", "junk line", "2024 INFO a b"]))
        else:
            ls.append(f"2024-0{rng.randint(1, 9)}-1{rng.randint(0, 9)}T0{rng.randint(0, 9)}:{rng.randint(10, 59)}:{rng.randint(10, 59)} {rng.choice(['DEBUG', 'INFO', 'WARN', 'ERROR', 'ERROR'])} {rng.choice(['web', 'db', 'auth', 'cache'])} {rng.choice(['ok', 'failed to connect', '', 'x=1 y=2'])}")
    return _c(ls)


RX = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}) (DEBUG|INFO|WARN|ERROR) (\S+)(?: (.*))?$")


def solve(case):
    lv, comp, ts, errs, mal = Counter(), Counter(), [], [], 0
    for ln in case["stdin"].split("\n")[:-1]:
        m = RX.match(ln)
        if not m:
            mal += 1
            continue
        t, l, c, msg = m.group(1), m.group(2), m.group(3), m.group(4) or ""
        lv[l] += 1
        comp[c] += 1
        ts.append(t)
        if l == "ERROR":
            errs.append(f"  [{c}] {msg}")
    out = ["levels: " + " ".join(f"{k}={lv[k]}" for k in ("DEBUG", "INFO", "WARN", "ERROR")), "top components:"]
    for c, n in sorted(comp.items(), key=lambda kv: (-kv[1], kv[0]))[:3]:
        out.append(f"  {c} {n}")
    out.append(f"first: {min(ts) if ts else '-'}")
    out.append(f"last: {max(ts) if ts else '-'}")
    out.append("errors:")
    out += errs
    out.append(f"malformed: {mal}")
    return "\n".join(out) + "\n"
