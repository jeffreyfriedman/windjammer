def _c(stdin): return {"stdin": stdin}
SAMPLES = [
    _c("pkg app 1\npkg lib 3\npkg core 2\ndep app lib >= 2\ndep app core = 2\ndep lib core < 5\ninstall app\n"),
    _c("pkg a 1\npkg b 2\npkg c 3\ndep a b >= 3\ndep b c = 3\ndep c a > 0\ndep c d = 1\ninstall a\n"),
]
EDGE = [
    _c(""),
    _c("pkg a 1\n"),
    _c("pkg a 1\ndep a a = 1\ninstall a\n"),
    _c("pkg x 1\npkg y 1\npkg z 1\ndep x y >= 1\ndep y z >= 1\ndep z x >= 1\ninstall x\ninstall y\n"),
    _c("pkg top 1\npkg b 1\npkg c 1\npkg d 1\ndep top c >= 1\ndep top b >= 1\ndep b d >= 1\ndep c d >= 1\ninstall top\ninstall d\ninstall c\n"),
    _c("pkg a 1\npkg a 2\ndep b a = 1\npkg z\nfoo bar\ndep a z\ndep a z ~ 1\ndep a z >= x\ninstall q\ninstall a\n\n"),
    _c("pkg a 5\npkg b 5\ndep a b > 5\ndep a b >= 6\ndep a b <= 4\ndep a b = 5\ndep a b < 5\ninstall a\n"),
    _c("pkg a 1\npkg m 7\npkg k 8\ndep a m >= 7\ndep a k <= 7\ndep k m >= 007\ndep m a = 1\ninstall a\ninstall k\n"),
    _c("pkg a 1\ndep a w = 1\ndep a v = 1\npkg c 1\ndep c a = 1\ninstall c\ninstall a\n"),
    _c("install x\ndep x y = 1\ninstall x\n"),
]
NAMES = ["app", "lib", "core", "net", "ui", "db", "log"]
def rand(rng):
    lines, vers = [], {}
    names = rng.sample(NAMES, rng.randint(3, 7))
    for n in names:
        if rng.random() < 0.98:
            vers[n] = rng.randint(1, 6); lines.append(f"pkg {n} {vers[n]}")
    for _ in range(rng.randint(2, 12)):
        i, j = rng.randrange(len(names)), rng.randrange(len(names))
        if rng.random() < 0.96 and i > j: i, j = j, i
        v = vers.get(names[j], 3)
        op = rng.choice([">=", "<=", "=", ">", "<"])
        if rng.random() < 0.95:
            op, val = (">=", max(0, v - rng.randint(0, 2))) if rng.random() < 0.6 else ("<=", v + rng.randint(0, 2))
        else:
            val = rng.randint(1, 6)
        lines.append(f"dep {names[i]} {names[j]} {op} {val}")
    for _ in range(rng.randint(1, 3)):
        lines.append(f"install {rng.choice(names)}")
    if rng.random() < 0.08: lines.insert(rng.randint(0, len(lines)), rng.choice(["junk", "pkg x", "dep a b >= 2x", ""]))
    if rng.random() < 0.7: rng.shuffle(lines)
    return _c("\n".join(lines) + "\n")
def _num(s): return s.isascii() and s.isdigit() and len(s) <= 6
def solve(case):
    errs, pkgs, deps, targets = [], {}, [], []
    for n, ln in enumerate(case["stdin"].split("\n"), 1):
        if ln == "": continue
        p = ln.split(" ")
        if "" in p: errs.append(f"error: bad line {n}"); continue
        if p[0] == "pkg" and len(p) == 3 and _num(p[2]):
            if p[1] in pkgs: errs.append(f"error: duplicate package {p[1]}")
            else: pkgs[p[1]] = int(p[2])
        elif p[0] == "dep" and len(p) == 5 and p[3] in (">=", "<=", "=", ">", "<") and _num(p[4]):
            deps.append((p[1], p[2], p[3], int(p[4])))
        elif p[0] == "install" and len(p) == 2:
            targets.append((n, p[1]))
        else:
            errs.append(f"error: bad line {n}")
    good = []
    for d in deps:
        if d[0] not in pkgs: errs.append(f"error: dep for unknown package {d[0]}")
        else: good.append(d)
    state, order, stack = {}, [], []
    def sat(v, op, w):
        return {">=": v >= w, "<=": v <= w, "=": v == w, ">": v > w, "<": v < w}[op]
    def visit(u):
        state[u] = 1; stack.append(u)
        for d in sorted({x[1] for x in good if x[0] == u}):
            if d not in pkgs:
                errs.append(f"error: missing dependency {d} of {u}")
                continue
            for (a, b, op, w) in good:
                if a == u and b == d and not sat(pkgs[d], op, w):
                    errs.append(f"error: {u} needs {d} {op} {w} but found {pkgs[d]}")
            if state.get(d) == 1:
                cyc = stack[stack.index(d):] + [d]
                errs.append("error: cycle " + " -> ".join(cyc))
            elif d not in state:
                visit(d)
        stack.pop(); state[u] = 2; order.append(u)
    for _, t in targets:
        if t not in pkgs: errs.append(f"error: unknown package {t}")
        elif t not in state: visit(t)
    if errs:
        return "".join(e + "\n" for e in errs), 1
    return "".join(f"install {u} {pkgs[u]}\n" for u in order)
