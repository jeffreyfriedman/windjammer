def _c(w, xs):
    return {"stdin": f"{w} {len(xs)}\n" + " ".join(map(str, xs)) + "\n"}


SAMPLES = [_c(2, [1, 2, 3, 4, 5]), _c(3, [10, -10, 7])]
EDGE = [_c(1, []), _c(16, [1, 2, 3]), _c(4, [10 ** 9] * 100000), _c(3, [-10 ** 9] * 7), _c(5, list(range(10)))]


def rand(rng):
    n = rng.randint(0, 300)
    return _c(rng.randint(1, 16), [rng.randint(-10 ** 9, 10 ** 9) for _ in range(n)])


def solve(case):
    t = case["stdin"].split()
    w, n = int(t[0]), int(t[1])
    xs = list(map(int, t[2:2 + n]))
    q, r = divmod(n, w)
    out, pos = [], 0
    for i in range(w):
        k = q + 1 if i < r else q
        out.append(f"chunk {i}: {sum(xs[pos:pos + k])}")
        pos += k
    out.append(f"total: {sum(xs)}")
    return "\n".join(out) + "\n"
