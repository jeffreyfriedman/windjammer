def _c(iv):
    return {"stdin": f"{len(iv)}\n" + "".join(f"{a} {b}\n" for a, b in iv)}


SAMPLES = [_c([(1, 3), (2, 6), (8, 10), (15, 18)]), _c([(1, 4), (4, 5)])]
EDGE = [_c([]), _c([(5, 5)]), _c([(1, 3), (4, 5)]), _c([(-10, 10), (-5, -4), (0, 0), (10, 20)]), _c([(3, 4), (1, 2), (2, 3)])]


def rand(rng):
    iv = []
    for _ in range(rng.randint(1, 15)):
        a = rng.randint(-30, 30)
        iv.append((a, a + rng.randint(0, 8)))
    return _c(iv)


def solve(case):
    xs = list(map(int, case["stdin"].split()))
    iv = sorted((xs[1 + 2 * i], xs[2 + 2 * i]) for i in range(xs[0]))
    m = []
    for a, b in iv:
        if m and a <= m[-1][1]:
            m[-1][1] = max(m[-1][1], b)
        else:
            m.append([a, b])
    return "".join(f"{a} {b}\n" for a, b in m)
