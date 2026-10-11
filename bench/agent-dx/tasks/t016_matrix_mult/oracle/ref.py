def fmt(cases):
    s = f"{len(cases)}\n"
    for a, b in cases:
        for m in (a, b):
            s += f"{len(m)} {len(m[0])}\n" + "".join(" ".join(map(str, r)) + "\n" for r in m)
    return {"stdin": s}


SAMPLES = [fmt([([[1, 2], [3, 4]], [[5, 6], [7, 8]])]), fmt([([[1, 2, 3]], [[1], [2], [3]]), ([[1]], [[1, 2]]), ([[1, 2]], [[1, 2]])])]
EDGE = [fmt([([[1000] * 6] * 6, [[-1000] * 6] * 6)]), fmt([([[0]], [[0]])]), fmt([([[1, 0], [0, 1]], [[9, -9], [3, 3]])])]


def rand(rng):
    cases = []
    for _ in range(rng.randint(1, 4)):
        r1, c1 = rng.randint(1, 6), rng.randint(1, 6)
        r2 = c1 if rng.random() < 0.8 else rng.randint(1, 6)
        c2 = rng.randint(1, 6)
        g = lambda r, c: [[rng.randint(-1000, 1000) if rng.random() < 0.5 else rng.randint(-3, 3) for _ in range(c)] for _ in range(r)]
        cases.append((g(r1, c1), g(r2, c2)))
    return fmt(cases)


def solve(case):
    toks = case["stdin"].split()
    pos = 0

    def nxt():
        nonlocal pos
        pos += 1
        return int(toks[pos - 1])

    def mat():
        r, c = nxt(), nxt()
        return [[nxt() for _ in range(c)] for _ in range(r)]
    out = []
    for _ in range(nxt()):
        a, b = mat(), mat()
        if len(a[0]) != len(b):
            out.append("dimension mismatch")
        else:
            for i in range(len(a)):
                out.append(" ".join(str(sum(a[i][k] * b[k][j] for k in range(len(b)))) for j in range(len(b[0]))))
        out.append("---")
    return "\n".join(out) + "\n"
