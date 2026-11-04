import datetime
def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("2024-01-01 2024-03-01\n2000-02-28 2000-03-01\n"), _c("2023-12-31 2023-12-31\nnope\n")]
EDGE = [_c("1900-01-01 2100-12-31\n"), _c("2100-02-29 2100-03-01\n2024-02-29 2023-02-28\n"), _c("2024-13-01 2024-01-01\n2024-04-31 2024-05-01\n"), _c("\n2024-1-1 2024-01-02\n")]
def rand(rng):
    ls = []
    for _ in range(rng.randint(1, 6)):
        def d():
            y, m, dd = rng.randint(1900, 2100), rng.randint(1, 12), rng.randint(1, 31)
            return f"{y:04d}-{m:02d}-{dd:02d}"
        ls.append(f"{d()} {d()}")
    return _c("\n".join(ls) + "\n")
def _p(s):
    if len(s) != 10 or s[4] != "-" or s[7] != "-" or not (s[:4] + s[5:7] + s[8:]).isdigit(): return None
    y, m, d = int(s[:4]), int(s[5:7]), int(s[8:])
    if not 1900 <= y <= 2100: return None
    try: return datetime.date(y, m, d)
    except ValueError: return None
def solve(case):
    out = []
    for ln in case["stdin"].splitlines():
        if not ln.strip(): continue
        p = ln.split(" ")
        a = _p(p[0]) if len(p) == 2 else None
        b = _p(p[1]) if len(p) == 2 else None
        if a is None or b is None: out.append("invalid\n"); continue
        lo = min(a, b)
        out.append(f"{abs((a - b).days)} {lo.strftime('%A')}\n")
    return "".join(out)
