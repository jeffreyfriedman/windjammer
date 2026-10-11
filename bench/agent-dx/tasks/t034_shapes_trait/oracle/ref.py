def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["square 3", "rect 2 5", "rtri 3 4"]), _c(["trap 2 4 3"])]
EDGE = [_c([]), _c(["square 1000000000", "rect 1000000000 1000000000"]), _c(["rtri 2 3", "rect 1 3", "square 1"])]


def rand(rng):
    ls = []
    for _ in range(rng.randint(1, 8)):
        k = rng.choice(["square", "rect", "rtri", "trap"])
        n = {"square": 1, "rect": 2, "rtri": 2, "trap": 3}[k]
        ls.append(k + "".join(f" {rng.randint(1, 50)}" for _ in range(n)))
    return _c(ls)


def solve(case):
    out, tot, best = [], 0, None
    for ln in case["stdin"].split("\n")[:-1]:
        p = ln.split()
        v = list(map(int, p[1:]))
        if p[0] == "square":
            c, d = 4, 2 * v[0] * v[0]
        elif p[0] == "rect":
            c, d = 4, 2 * v[0] * v[1]
        elif p[0] == "rtri":
            c, d = 3, v[0] * v[1]
        else:
            c, d = 4, (v[0] + v[1]) * v[2]
        out.append(f"{p[0]} corners={c} area2={d}")
        tot += d
        if best is None or d > best[1]:
            best = (p[0], d)
    out.append(f"total area2={tot}")
    out.append(f"largest={best[0]} area2={best[1]}" if best else "largest=none")
    return "\n".join(out) + "\n"
