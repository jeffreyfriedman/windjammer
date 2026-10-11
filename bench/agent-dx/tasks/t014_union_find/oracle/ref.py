def _c(n, cmds):
    return {"stdin": f"{n}\n" + "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(5, ["union 0 1", "union 1 2", "find 0 2", "find 0 4", "size 2", "union 0 2"]), _c(1, ["size 0"])]
EDGE = [_c(1000, ["union 0 999", "size 999", "find 0 500"]), _c(3, [])]


def rand(rng):
    n = rng.randint(1, 30)
    cmds = []
    for _ in range(rng.randint(1, 60)):
        a, b = rng.randrange(n), rng.randrange(n)
        r = rng.random()
        cmds.append(f"union {a} {b}" if r < 0.5 else f"find {a} {b}" if r < 0.8 else f"size {a}")
    return _c(n, cmds)


def solve(case):
    lines = case["stdin"].strip("\n").split("\n")
    n = int(lines[0])
    par = list(range(n))

    def f(x):
        while par[x] != x:
            par[x] = par[par[x]]
            x = par[x]
        return x
    out = []
    for ln in lines[1:]:
        p = ln.split()
        if p[0] == "union":
            a, b = f(int(p[1])), f(int(p[2]))
            if a == b:
                out.append("same")
            else:
                par[a] = b
                out.append("merged")
        elif p[0] == "find":
            out.append("yes" if f(int(p[1])) == f(int(p[2])) else "no")
        else:
            r = f(int(p[1]))
            out.append(str(sum(1 for i in range(n) if f(i) == r)))
    out.append(f"components {len({f(i) for i in range(n)})}")
    return "\n".join(out) + "\n"
