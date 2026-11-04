def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("ann login 5\nbob login 3\nann edit 40\nann edit 2\nbob logout 1\n"), _c("x a 1\nbad line\ny b -4\n")]
EDGE = [_c(""), _c("u b 1\nu a 1\n"), _c("u a 0\nv a 0\n"), _c("only two\n\nu a 1 extra\nu a 7\n"), _c("u a 10x\n")]
def rand(rng):
    users, acts = ["ann", "bob", "cy", "dee"], ["a", "b", "c"]
    lines = []
    for _ in range(rng.randint(1, 14)):
        r = rng.random()
        if r < 0.1: lines.append("junk")
        elif r < 0.15: lines.append(f"{rng.choice(users)} {rng.choice(acts)} x")
        else: lines.append(f"{rng.choice(users)} {rng.choice(acts)} {rng.randint(0, 50)}")
    return _c("\n".join(lines) + "\n")
def solve(case):
    st, bad = {}, 0
    for ln in case["stdin"].splitlines():
        p = ln.split(" ")
        if len(p) != 3 or not p[2].isdigit() or not p[2].isascii():
            bad += 1; continue
        u = st.setdefault(p[0], {"n": 0, "t": 0, "a": {}})
        u["n"] += 1; u["t"] += int(p[2]); u["a"][p[1]] = u["a"].get(p[1], 0) + 1
    out = []
    for name in sorted(st, key=lambda n: (-st[n]["t"], n)):
        u = st[name]
        top = sorted(u["a"], key=lambda a: (-u["a"][a], a))[0]
        out.append(f"{name} {u['n']} {u['t']} {top}\n")
    return "".join(out) + f"malformed: {bad}\n"
