import heapq


def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["push 5 a", "push 1 b", "push 3 c", "pop", "peek", "size", "pop", "pop", "pop"]), _c(["push 2 x", "push 2 y", "pop", "pop"])]
EDGE = [_c(["pop", "peek", "size"]), _c(["push -5 neg", "push 0 zero", "push -5 neg2", "pop", "pop", "pop"]),
        _c([f"push {i % 7} n{i}" for i in range(60)] + ["pop"] * 62)]


def rand(rng):
    cmds, n = [], 0
    for _ in range(rng.randint(10, 80)):
        r = rng.random()
        if r < 0.55:
            cmds.append(f"push {rng.randint(-5, 5)} w{n}")
            n += 1
        else:
            cmds.append(rng.choice(["pop", "pop", "peek", "size"]))
    return _c(cmds)


def solve(case):
    h, seq, out = [], 0, []
    for ln in case["stdin"].strip().split("\n"):
        p = ln.split()
        if p[0] == "push":
            heapq.heappush(h, (int(p[1]), seq, p[2]))
            seq += 1
        elif p[0] == "pop":
            if h:
                a, _, b = heapq.heappop(h)
                out.append(f"{a} {b}")
            else:
                out.append("empty")
        elif p[0] == "peek":
            out.append(f"{h[0][0]} {h[0][2]}" if h else "empty")
        else:
            out.append(str(len(h)))
    return "\n".join(out) + "\n"
