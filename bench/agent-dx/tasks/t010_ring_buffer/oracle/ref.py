def _c(cap, cmds):
    return {"stdin": f"{cap}\n" + "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(2, ["push 1", "push 2", "push 3", "dump", "pop", "size"]), _c(3, ["pop", "peek", "dump"])]
EDGE = [_c(1, ["push 5", "push 6", "pop", "push 7", "peek", "dump", "pop", "pop"]),
        _c(3, ["push -1", "push 0", "push 9", "pop", "push 4", "push 5", "dump", "size"])]


def rand(rng):
    cap = rng.randint(1, 6)
    cmds = []
    for _ in range(rng.randint(5, 40)):
        r = rng.random()
        cmds.append(f"push {rng.randint(-50, 50)}" if r < 0.45 else
                    "pop" if r < 0.7 else rng.choice(["peek", "size", "dump"]))
    return _c(cap, cmds)


def solve(case):
    lines = case["stdin"].strip().split("\n")
    cap, q, out = int(lines[0]), [], []
    for ln in lines[1:]:
        p = ln.split()
        if p[0] == "push":
            if len(q) >= cap:
                out.append("full")
            else:
                q.append(int(p[1]))
                out.append("ok")
        elif p[0] == "pop":
            out.append(str(q.pop(0)) if q else "empty")
        elif p[0] == "peek":
            out.append(str(q[0]) if q else "empty")
        elif p[0] == "size":
            out.append(str(len(q)))
        else:
            out.append(" ".join(map(str, q)) if q else "<empty>")
    return "\n".join(out) + "\n"
