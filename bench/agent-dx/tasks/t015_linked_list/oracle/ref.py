def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["push_back 1", "push_back 2", "push_front 0", "print", "reverse", "print", "nth 1", "nth 7"]),
           _c(["pop_front", "pop_back", "remove 3", "print"])]
EDGE = [_c(["push_back 1", "pop_back", "pop_front", "print", "push_front 4", "remove 4", "print"]),
        _c(["push_back 2", "push_back 2", "push_back 3", "remove 2", "print", "reverse", "print", "nth 0", "nth -1"])]


def rand(rng):
    cmds = []
    for _ in range(rng.randint(8, 50)):
        r = rng.random()
        x = rng.randint(-5, 5)
        cmds.append(f"push_front {x}" if r < 0.2 else f"push_back {x}" if r < 0.4 else
                    rng.choice(["pop_front", "pop_back", "reverse", "print", "print"]) if r < 0.7 else
                    f"remove {x}" if r < 0.85 else f"nth {rng.randint(-1, 6)}")
    return _c(cmds)


def solve(case):
    q, out = [], []
    for ln in case["stdin"].strip().split("\n"):
        p = ln.split()
        c = p[0]
        if c == "push_front":
            q.insert(0, int(p[1]))
        elif c == "push_back":
            q.append(int(p[1]))
        elif c == "pop_front":
            out.append(str(q.pop(0)) if q else "empty")
        elif c == "pop_back":
            out.append(str(q.pop()) if q else "empty")
        elif c == "remove":
            x = int(p[1])
            if x in q:
                q.remove(x)
                out.append("removed")
            else:
                out.append("missing")
        elif c == "reverse":
            q.reverse()
        elif c == "print":
            out.append(" ".join(map(str, q)) if q else "<empty>")
        else:
            k = int(p[1])
            out.append(str(q[k]) if 0 <= k < len(q) else "none")
    return "\n".join(out) + ("\n" if out else "")
