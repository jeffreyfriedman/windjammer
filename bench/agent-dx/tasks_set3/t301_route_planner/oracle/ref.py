def _c(stdin): return {"stdin": stdin}
SAMPLES = [
    _c("edge s a 1\nedge s b 1\nedge a t 1\nedge b t 1\npath s t\npath t s\npath s q\n"),
    _c("edge a b 5\nedge b c 5\nedge a c 20\npath a c\nedge a c 3\npath a c\nedge a b 0\nhello\n"),
]
EDGE = [
    _c(""),
    _c("edge s n2 1\nedge s n10 1\nedge n2 t 1\nedge n10 t 1\npath s t\n"),
    _c("edge s a 2\nedge s B 2\nedge a t 1\nedge B t 1\npath s t\n"),
    _c("edge x y 4\nedge x y 9\nedge x y 2\npath x y\npath y x\npath x x\npath y y\n"),
    _c("edge a a 3\npath a a\npath a b\npath b a\npath q z\n"),
    _c("edge a b -1\nedge a b 0\nedge a b 1000001\nedge a b 1000000\nedge a b 1x\nedge a b\nedge a\n path a b\npath a b\n \nedge a b 5 6\nfoo bar\n"),
    _c("edge a d 10\nedge a b 3\nedge b c 3\nedge c d 3\nedge a c 6\npath a d\n\n\npath d a\n"),
    _c("edge a b 1\nedge a c 1\nedge b d 2\nedge c d 2\nedge b e 1\nedge c e 1\nedge e d 1\npath a d\npath a e\n"),
    _c("edge 1 2 1\nedge 2 3 1\nedge 1 3 2\nedge 1 10 1\nedge 10 3 1\npath 1 3\n"),
]
NODES = ["a", "b", "c", "d", "e", "f", "n2", "n10", "B", "A1"]
def rand(rng):
    nodes = rng.sample(NODES, rng.randint(3, 7))
    lines = []
    for _ in range(rng.randint(8, 22)):
        r = rng.random()
        if r < 0.55:
            lines.append(f"edge {rng.choice(nodes)} {rng.choice(nodes)} {rng.randint(1, 3) if rng.random() < 0.8 else rng.randint(1, 9)}")
        elif r < 0.9:
            lines.append(f"path {rng.choice(nodes + ['zz'] if rng.random() < 0.1 else nodes)} {rng.choice(nodes)}")
        elif r < 0.95:
            lines.append(rng.choice(["", "edge a b 0", "path a", "nope", "edge a b x"]))
        else:
            lines.append(f"edge {rng.choice(nodes)} {rng.choice(nodes)} {rng.choice(['0', '-2', 'q'])}")
    lines.append(f"path {rng.choice(nodes)} {rng.choice(nodes)}")
    return _c("\n".join(lines) + "\n")
def _valid_w(w):
    return w.isascii() and w.isdigit() and 1 <= int(w) <= 1000000
def solve(case):
    edges, known, out = {}, set(), []
    for ln in case["stdin"].split("\n")[:-1] if case["stdin"].endswith("\n") else case["stdin"].split("\n"):
        if ln == "":
            continue
        p = ln.split(" ")
        if p[0] == "edge" and len(p) == 4 and "" not in p and _valid_w(p[3]):
            a, b, w = p[1], p[2], int(p[3])
            known.update([a, b])
            edges[(a, b)] = min(w, edges.get((a, b), w))
        elif p[0] == "path" and len(p) == 3 and "" not in p:
            a, b = p[1], p[2]
            if a not in known: out.append(f"unknown node {a}")
            elif b not in known: out.append(f"unknown node {b}")
            elif a == b: out.append(f"0 {a}")
            else:
                best = []
                def dfs(u, path, cost):
                    if u == b:
                        best.append((cost, list(path))); return
                    for (x, y), w in edges.items():
                        if x == u and y not in path:
                            path.append(y); dfs(y, path, cost + w); path.pop()
                dfs(a, [a], 0)
                if not best: out.append("unreachable")
                else:
                    c, pth = min(best)
                    out.append(f"{c} " + ">".join(pth))
        else:
            out.append(f"bad command: {ln}")
    return "".join(x + "\n" for x in out)
