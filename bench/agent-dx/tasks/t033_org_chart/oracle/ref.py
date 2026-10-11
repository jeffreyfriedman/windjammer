def _c(recs, qs):
    return {"stdin": f"{len(recs)}\n" + "".join(f"{n} {m} {s}\n" for n, m, s in recs) + "".join(q + "\n" for q in qs)}


R = [("ann", "-", 300), ("bob", "ann", 200), ("cat", "bob", 100), ("dan", "cat", 90), ("eve", "zed", 50)]
SAMPLES = [_c(R, ["cat", "dan"]), _c(R, ["ann", "bob", "nobody"])]
EDGE = [_c([], ["x"]), _c(R, ["eve", "dan", "cat", "bob", "ann"]), _c([("a", "-", -5), ("b", "a", 0), ("c", "b", 7)], ["c", "b", "a"])]


def rand(rng):
    names = [f"p{i}" for i in range(8)]
    recs = []
    for i, n in enumerate(names):
        m = "-" if i == 0 or rng.random() < 0.15 else rng.choice(names[:i] + (["ghost"] if rng.random() < 0.1 else []))
        recs.append((n, m, rng.randint(-100, 1000)))
    return _c(recs, [rng.choice(names + ["nobody"]) for _ in range(rng.randint(1, 8))])


def solve(case):
    lines = case["stdin"].split("\n")[:-1]
    n = int(lines[0])
    d = {}
    for ln in lines[1:1 + n]:
        a, m, s = ln.split()
        d[a] = (None if m == "-" else m, int(s))
    out = []
    for q in lines[1 + n:]:
        if q not in d:
            out.append(f"none: unknown {q}")
            continue
        m1 = d[q][0]
        if m1 is None:
            out.append(f"none: no manager {q}")
        elif m1 not in d:
            out.append(f"none: unknown {m1}")
        elif d[m1][0] is None:
            out.append(f"none: no skip-level {m1}")
        elif d[m1][0] not in d:
            out.append(f"none: unknown {d[m1][0]}")
        else:
            out.append(str(d[d[m1][0]][1]))
    return "\n".join(out) + ("\n" if out else "")
