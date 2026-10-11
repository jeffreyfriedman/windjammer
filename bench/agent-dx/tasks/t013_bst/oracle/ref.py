def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["insert 5", "insert 3", "insert 8", "insert 3", "inorder", "height", "contains 8", "min", "max"]),
           _c(["inorder", "height", "min", "delete 1"])]
EDGE = [_c([f"insert {i}" for i in range(1, 8)] + ["height", "delete 1", "delete 4", "inorder", "height", "delete 99"]),
        _c(["insert 4", "insert 2", "insert 6", "insert 5", "insert 7", "delete 4", "inorder", "height", "min", "max"])]


def rand(rng):
    cmds = []
    for _ in range(rng.randint(10, 60)):
        r = rng.random()
        x = rng.randint(-15, 15)
        cmds.append(f"insert {x}" if r < 0.4 else f"contains {x}" if r < 0.55 else f"delete {x}" if r < 0.7
                    else rng.choice(["inorder", "height", "min", "max"]))
    return _c(cmds)


class N:
    def __init__(self, k):
        self.k, self.l, self.r = k, None, None


def ins(n, k):
    if n is None:
        return N(k), True
    if k == n.k:
        return n, False
    if k < n.k:
        n.l, ok = ins(n.l, k)
    else:
        n.r, ok = ins(n.r, k)
    return n, ok


def dele(n, k):
    if n is None:
        return None, False
    if k < n.k:
        n.l, ok = dele(n.l, k)
        return n, ok
    if k > n.k:
        n.r, ok = dele(n.r, k)
        return n, ok
    if n.l is None:
        return n.r, True
    if n.r is None:
        return n.l, True
    s = n.r
    while s.l:
        s = s.l
    n.k = s.k
    n.r, _ = dele(n.r, s.k)
    return n, True


def inorder(n):
    return inorder(n.l) + [n.k] + inorder(n.r) if n else []


def height(n):
    return 0 if n is None else 1 + max(height(n.l), height(n.r))


def solve(case):
    root, out = None, []
    for ln in case["stdin"].strip().split("\n"):
        p = ln.split()
        c = p[0]
        if c == "insert":
            root, ok = ins(root, int(p[1]))
            out.append("ok" if ok else "dup")
        elif c == "contains":
            out.append("yes" if int(p[1]) in inorder(root) else "no")
        elif c == "delete":
            root, ok = dele(root, int(p[1]))
            out.append("ok" if ok else "missing")
        elif c == "inorder":
            k = inorder(root)
            out.append(" ".join(map(str, k)) if k else "<empty>")
        elif c == "height":
            out.append(str(height(root)))
        else:
            k = inorder(root)
            out.append(str(k[0] if c == "min" else k[-1]) if k else "empty")
    return "\n".join(out) + "\n"
