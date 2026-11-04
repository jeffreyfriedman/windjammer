def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("add a 3\nadd b 5\nadd c 5\npop\npeek\nsize\npop\npop\npop\n"), _c("add x -1\nadd y -1\nsize\n")]
EDGE = [_c(""), _c("pop\npeek\nsize\n"), _c("add a\nfoo\nadd b x\nadd c 1 2\n"), _c("add a 0\nadd b 0\nadd c 0\npop\nadd d 0\n")]
def rand(rng):
    cmds = []
    for _ in range(rng.randint(1, 16)):
        r = rng.random()
        if r < 0.5: cmds.append(f"add {rng.choice('abcdef')}{rng.randint(0,9)} {rng.randint(-3, 4)}")
        elif r < 0.7: cmds.append("pop")
        elif r < 0.8: cmds.append("peek")
        elif r < 0.9: cmds.append("size")
        else: cmds.append(rng.choice(["boom", "add z", "pop now"]))
    return _c("\n".join(cmds) + "\n")
def solve(case):
    q, n, out = [], 0, []
    def best():
        return min(range(len(q)), key=lambda i: (-q[i][1], q[i][2]))
    for ln in case["stdin"].splitlines():
        p = ln.split(" ")
        if p[0] == "add" and len(p) == 3 and (p[2].lstrip("-").isdigit() and p[2].lstrip("-") != "" and (p[2].count("-") <= 1) and not p[2].endswith("-")):
            q.append((p[1], int(p[2]), n)); n += 1
        elif p == ["pop"]:
            if not q: out.append("empty\n")
            else: out.append(q.pop(best())[0] + "\n")
        elif p == ["peek"]:
            out.append((q[best()][0] if q else "empty") + "\n")
        elif p == ["size"]:
            out.append(f"{len(q)}\n")
        else: out.append("error\n")
    out.append("left:\n")
    for name, pr, _ in sorted(q, key=lambda t: (-t[1], t[2])): out.append(f"{name} {pr}\n")
    return "".join(out)
