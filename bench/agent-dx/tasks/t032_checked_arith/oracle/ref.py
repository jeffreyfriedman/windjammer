LO, HI = -2 ** 31, 2 ** 31 - 1


def _c(rows):
    return {"stdin": "".join(f"{o} {a} {b}\n" for o, a, b in rows)}


SAMPLES = [_c([("add", 1, 2), ("mul", 65536, 65536), ("pow", 2, 10), ("div", 7, 0)]), _c([("sub", LO, 1), ("rem", -7, 3), ("pow", 2, -1)])]
EDGE = [_c([("add", HI, 1), ("sub", LO, -1), ("mul", LO, -1), ("div", LO, -1), ("rem", LO, -1), ("pow", 0, 0), ("pow", -1, 2000000000), ("pow", -1, 2000000001), ("pow", 1, 2000000000),
            ("pow", 0, 2000000000), ("pow", -2, 31), ("pow", -2, 32), ("pow", 2, 30), ("pow", 2, 31), ("div", 0, 0), ("div", -7, 2), ("rem", 7, -3), ("mul", 0, LO)])]


def rand(rng):
    rows = []
    for _ in range(rng.randint(3, 12)):
        op = rng.choice(["add", "sub", "mul", "pow", "div", "rem"])
        big = lambda: rng.choice([rng.randint(LO, HI), rng.randint(-100, 100), HI, LO, 0, 1, -1])
        a, b = big(), big()
        if op == "pow":
            b = rng.choice([rng.randint(-2, 40), rng.randint(0, 5)])
            a = rng.choice([rng.randint(-5, 5), rng.randint(-1000, 1000), 0, 1, -1, 2])
        rows.append((op, a, b))
    return _c(rows)


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        op, a, b = ln.split()
        a, b = int(a), int(b)
        if op == "add":
            r = a + b
        elif op == "sub":
            r = a - b
        elif op == "mul":
            r = a * b
        elif op in ("div", "rem"):
            if b == 0:
                out.append("divzero")
                continue
            q = abs(a) // abs(b)
            q = q if (a < 0) == (b < 0) else -q
            r = q if op == "div" else a - q * b
        else:
            if b < 0:
                out.append("invalid")
                continue
            if a in (0, 1):
                r = 1 if b == 0 else a
            elif a == -1:
                r = 1 if b % 2 == 0 else -1
            elif b > 40:
                r = HI + 1
            else:
                r = a ** b
        out.append(str(r) if LO <= r <= HI else "overflow")
    return "\n".join(out) + "\n"
