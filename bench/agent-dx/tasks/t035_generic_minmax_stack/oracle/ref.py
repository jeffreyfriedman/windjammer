def _c(kind, cmds):
    return {"stdin": kind + "\n" + "".join(c + "\n" for c in cmds)}


SAMPLES = [_c("int", ["push 3", "push 1", "push 2", "min", "max", "pop", "pop", "min", "size"]), _c("str", ["push pear", "push apple", "min", "max", "pop", "min"])]
EDGE = [_c("int", ["min", "pop", "size", "push -9223372036854775808", "push 9223372036854775807", "min", "max"]),
        _c("str", ["push ", "push a b", "push B", "min", "max", "size", "pop", "pop", "pop", "pop"])]


def rand(rng):
    kind = rng.choice(["int", "str"])
    cmds = []
    for _ in range(rng.randint(5, 40)):
        r = rng.random()
        if r < 0.5:
            cmds.append(f"push {rng.randint(-20, 20)}" if kind == "int" else "push " + "".join(rng.choice("abAB z") for _ in range(rng.randint(0, 4))))
        else:
            cmds.append(rng.choice(["pop", "min", "max", "size", "pop"]))
    return _c(kind, cmds)


def solve(case):
    lines = case["stdin"].split("\n")[:-1]
    kind, st, out = lines[0], [], []
    for ln in lines[1:]:
        if ln.startswith("push"):
            x = ln[5:]
            st.append(int(x) if kind == "int" else x)
        elif ln == "pop":
            out.append(str(st.pop()) if st else "empty")
        elif ln == "size":
            out.append(str(len(st)))
        else:
            out.append(str(min(st) if ln == "min" else max(st)) if st else "empty")
    return "\n".join(out) + ("\n" if out else "")
