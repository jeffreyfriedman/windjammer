import math


def _c(pairs):
    return {"stdin": f"{len(pairs)}\n" + "".join(f"{a} {b}\n" for a, b in pairs)}


SAMPLES = [_c([(12, 18)]), _c([(7, 5), (100, 25), (1, 1)])]
EDGE = [_c([(1, 1000000000)]), _c([(1000000000, 1000000000)]), _c([(999999937, 999999929)]), _c([])]


def rand(rng):
    pairs = []
    for _ in range(rng.randint(1, 8)):
        g = rng.choice([1, 2, 6, 30, 1000])
        pairs.append((g * rng.randint(1, 10**9 // g), g * rng.randint(1, 10**9 // g)))
    return _c(pairs)


def solve(case):
    lines = case["stdin"].split("\n")
    n = int(lines[0])
    out = []
    for i in range(n):
        a, b = map(int, lines[1 + i].split())
        g = math.gcd(a, b)
        out.append(f"{g} {a // g * b}")
    return "\n".join(out) + ("\n" if out else "")
