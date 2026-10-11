D = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"


def enc(n, b):
    if n == 0:
        return "0"
    s = ""
    while n:
        s = D[n % b] + s
        n //= b
    return s


def _c(rows):
    return {"stdin": "".join(f"{v} {f} {t}\n" for v, f, t in rows)}


SAMPLES = [_c([("255", 10, 16)]), _c([("1010", 2, 10), ("ZZ", 36, 2), ("0", 7, 3)])]
EDGE = [_c([(enc(2 ** 63 - 1, 10), 10, 2), (enc(2 ** 63 - 1, 36), 36, 10), ("0", 2, 2), ("1", 36, 2)])]


def rand(rng):
    rows = []
    for _ in range(rng.randint(1, 8)):
        f, t = rng.randint(2, 36), rng.randint(2, 36)
        n = rng.choice([rng.randint(0, 100), rng.randint(0, 2 ** 63 - 1)])
        rows.append((enc(n, f), f, t))
    return _c(rows)


def solve(case):
    out = []
    for ln in case["stdin"].split("\n"):
        if not ln.strip():
            continue
        v, f, t = ln.split()
        out.append(enc(int(v, int(f)), int(t)))
    return "\n".join(out) + "\n"
