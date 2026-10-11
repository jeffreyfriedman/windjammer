def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["set a 10", "get a", "count 10", "begin", "set a 20", "get a", "rollback", "get a"]), _c(["rollback", "commit", "get zz"])]
EDGE = [_c(["set a 1", "begin", "set b 1", "begin", "delete a", "count 1", "rollback", "count 1", "commit", "rollback", "get b"]), _c(["begin", "set x 5", "begin", "set x 6", "commit", "get x", "count 6", "count 5"]),
        _c(["set a 1", "set a 2", "count 1", "count 2", "delete a", "delete a", "get a"])]


def rand(rng):
    cmds = []
    for _ in range(rng.randint(8, 50)):
        k, v = rng.choice("abc"), rng.randint(1, 3)
        r = rng.random()
        cmds.append(f"set {k} {v}" if r < 0.3 else f"get {k}" if r < 0.45 else f"delete {k}" if r < 0.55 else f"count {v}" if r < 0.7 else "begin" if r < 0.82 else "rollback" if r < 0.92 else "commit")
    return _c(cmds)


def solve(case):
    data, stack, out = {}, [], []
    for ln in case["stdin"].split("\n")[:-1]:
        p = ln.split()
        c = p[0]
        if c in ("set", "delete"):
            if stack:
                stack[-1].setdefault(p[1], data.get(p[1]))
            if c == "set":
                data[p[1]] = int(p[2])
            else:
                data.pop(p[1], None)
        elif c == "get":
            out.append(str(data[p[1]]) if p[1] in data else "NULL")
        elif c == "count":
            out.append(str(sum(1 for v in data.values() if v == int(p[1]))))
        elif c == "begin":
            stack.append({})
        elif c == "rollback":
            if not stack:
                out.append("NO TRANSACTION")
            else:
                for k, old in stack.pop().items():
                    if old is None:
                        data.pop(k, None)
                    else:
                        data[k] = old
        else:
            if not stack:
                out.append("NO TRANSACTION")
            stack = []
    return "\n".join(out) + ("\n" if out else "")
