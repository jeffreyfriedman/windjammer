def _c(w, xs):
    return {"stdin": f"{w} {len(xs)}\n" + " ".join(map(str, xs)) + "\n"}


SAMPLES = [_c(2, [1, 1, 2, 9, 0]), _c(3, [])]
EDGE = [_c(8, [9] * 50000), _c(1, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]), _c(8, [3, 3, 3])]


def rand(rng):
    return _c(rng.randint(1, 8), [rng.randint(0, 9) for _ in range(rng.randint(0, 400))])


def solve(case):
    t = case["stdin"].split()
    w, n = int(t[0]), int(t[1])
    xs = list(map(int, t[2:2 + n]))
    h = [0] * 10
    for x in xs:
        h[x] += 1
    out = ["hist: " + " ".join(map(str, h))]
    for i in range(w):
        out.append(f"worker {i}: {len(xs[i::w])}")
    out.append(f"checksum: {sum(d * h[d] for d in range(10))}")
    return "\n".join(out) + "\n"
