def _c(ns):
    return {"stdin": f"{len(ns)}\n" + "".join(f"{n}\n" for n in ns)}


SAMPLES = [_c([12321]), _c([0, 120, 987654321])]
EDGE = [_c([10 ** 18, 10 ** 18 - 1, 9, 10, 1000000000000000001 - 1]), _c([11, 101, 1001, 12])]


def rand(rng):
    out = []
    for _ in range(rng.randint(1, 8)):
        k = rng.randint(1, 18)
        n = rng.randint(0, 10 ** k)
        if rng.random() < 0.25:
            h = str(rng.randint(1, 10 ** 8))
            n = int(h + h[::-1][rng.randint(0, 1):])
        out.append(n)
    return _c(out)


def solve(case):
    ns = list(map(int, case["stdin"].split()))[1:]
    out = []
    for n in ns:
        s = str(n)
        ds = sum(map(int, s))
        r = n
        while r >= 10:
            r = sum(map(int, str(r)))
        out.append(f"{ds} {r} {int(s[::-1])} {'yes' if s == s[::-1] else 'no'}")
    return "\n".join(out) + "\n"
