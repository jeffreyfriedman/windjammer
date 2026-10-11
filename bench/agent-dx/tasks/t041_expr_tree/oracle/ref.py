def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["(+ 1 (* 2 3))", "42"]), _c(["(neg (- 5 7))", "(- -3 -4)"])]
EDGE = [_c(["0", "-5", "(neg (neg (neg 1)))", "(* (* (* 2 2) (* 2 2)) (* (* 2 2) (* 2 2)))", "(- (- 1 2) (- 3 (- 4 5)))"])]


def _gen(rng, d=0):
    if d > 4 or rng.random() < 0.3:
        return str(rng.randint(-9, 99))
    if rng.random() < 0.2:
        return f"(neg {_gen(rng, d + 1)})"
    return f"({rng.choice('+-*')} {_gen(rng, d + 1)} {_gen(rng, d + 1)})"


def rand(rng):
    return _c([_gen(rng) for _ in range(rng.randint(1, 6))])


def parse(toks, i):
    t = toks[i]
    if t == "(":
        op = toks[i + 1]
        a, j = parse(toks, i + 2)
        if op == "neg":
            return (op, a), j + 1
        b, j = parse(toks, j)
        return (op, a, b), j + 1
    return int(t), i + 1


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        toks = ln.replace("(", "( ").replace(")", " )").split()
        t, _ = parse(toks, 0)

        def ev(n):
            if isinstance(n, int):
                return n
            if n[0] == "neg":
                return -ev(n[1])
            a, b = ev(n[1]), ev(n[2])
            return a + b if n[0] == "+" else a - b if n[0] == "-" else a * b

        def dep(n):
            return 1 if isinstance(n, int) else 1 + max(dep(x) for x in n[1:])

        def cnt(n):
            return 1 if isinstance(n, int) else 1 + sum(cnt(x) for x in n[1:])

        def rd(n):
            if isinstance(n, int):
                return str(n)
            if n[0] == "neg":
                return f"-({rd(n[1])})"
            return f"({rd(n[1])} {n[0]} {rd(n[2])})"
        out.append(f"{ev(t)} {dep(t)} {cnt(t)} {rd(t)}")
    return "\n".join(out) + "\n"
