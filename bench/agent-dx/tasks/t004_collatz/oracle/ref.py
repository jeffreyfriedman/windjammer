def _c(ns):
    return {"stdin": f"{len(ns)}\n" + "".join(f"{n}\n" for n in ns)}


SAMPLES = [_c([1]), _c([6, 7, 27])]
EDGE = [_c([1000000]), _c([837799]), _c([999999, 2, 3, 4, 5]), _c([97, 871])]


def rand(rng):
    return _c([rng.randint(1, 1000000) for _ in range(rng.randint(1, 8))])


def solve(case):
    ns = list(map(int, case["stdin"].split()))[1:]
    out = []
    for n in ns:
        steps, peak = 0, n
        while n != 1:
            n = n // 2 if n % 2 == 0 else 3 * n + 1
            peak = max(peak, n)
            steps += 1
        out.append(f"{steps} {peak}")
    return "\n".join(out) + "\n"
