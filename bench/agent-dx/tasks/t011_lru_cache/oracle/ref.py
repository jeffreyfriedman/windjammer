from collections import OrderedDict


def _c(cap, cmds):
    return {"stdin": f"{cap}\n" + "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(2, ["put 1 1", "put 2 2", "get 1", "put 3 3", "get 2", "keys"]), _c(1, ["put 1 10", "put 2 20", "get 1", "keys"])]
EDGE = [_c(2, ["keys", "put 1 1", "put 1 5", "get 1", "keys", "put 2 2", "put 3 3", "keys"]),
        _c(3, ["put 1 1", "put 2 2", "put 3 3", "get 1", "put 4 4", "keys", "get 2", "get 3"])]


def rand(rng):
    cap = rng.randint(1, 5)
    cmds = []
    for _ in range(rng.randint(8, 50)):
        r = rng.random()
        k = rng.randint(1, 8)
        cmds.append(f"put {k} {rng.randint(-9, 99)}" if r < 0.5 else f"get {k}" if r < 0.9 else "keys")
    return _c(cap, cmds)


def solve(case):
    lines = case["stdin"].strip().split("\n")
    cap, d, out = int(lines[0]), OrderedDict(), []
    for ln in lines[1:]:
        p = ln.split()
        if p[0] == "put":
            k = int(p[1])
            d[k] = int(p[2])
            d.move_to_end(k)
            if len(d) > cap:
                ek, _ = d.popitem(last=False)
                out.append(f"evict {ek}")
        elif p[0] == "get":
            k = int(p[1])
            if k in d:
                d.move_to_end(k)
                out.append(str(d[k]))
            else:
                out.append("-1")
        else:
            out.append(" ".join(str(k) for k in reversed(d)) if d else "<empty>")
    return "\n".join(out) + ("\n" if out else "")
