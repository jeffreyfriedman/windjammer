def _c(ini, queries):
    return {"stdin": ini + "---\n" + "".join(q + "\n" for q in queries)}


SAMPLES = [_c("[a]\nx = 1\ny=2\n", ["a.x", "a.y", "a.z", "b.x"]), _c("g = top\n[s]\nk = v = w\n; c\n# d\n", ["g", "s.k", ".g", "s.c"])]
EDGE = [_c("", ["x"]), _c("[a]\nx=1\n[b]\nx=2\n[a]\nx=3\ny=4\n", ["a.x", "b.x", "a.y"]), _c("[s]\n k = spaced value \nnonsense line\n", ["s.k", "s.nonsense line"])]


def rand(rng):
    ini = ""
    secs = ["a", "b", "net"]
    keys = ["x", "y", "host", "port"]
    for _ in range(rng.randint(0, 12)):
        r = rng.random()
        if r < 0.25:
            ini += f"[{rng.choice(secs)}]\n"
        elif r < 0.8:
            ini += f"{rng.choice(keys)} = {rng.choice(['1', 'two', 'a=b', ' 3 ', ''])}\n"
        elif r < 0.9:
            ini += rng.choice([";", "#"]) + " comment\n"
        else:
            ini += "garbage\n"
    qs = [f"{rng.choice(secs + [''])}.{rng.choice(keys)}".lstrip(".") if rng.random() < 0.9 else rng.choice(keys) for _ in range(rng.randint(1, 8))]
    return _c(ini, qs)


def solve(case):
    ini, _, q = case["stdin"].partition("---\n")
    cur, d = "", {}
    for ln in ini.split("\n"):
        t = ln.strip()
        if not t or t[0] in ";#":
            continue
        if t.startswith("[") and t.endswith("]"):
            cur = t[1:-1]
        elif "=" in t:
            k, _, v = t.partition("=")
            d[(cur, k.strip())] = v.strip()
    out = []
    for qq in q.split("\n")[:-1]:
        s, dot, k = qq.partition(".")
        key = (s, k) if dot else ("", qq)
        out.append(d.get(key, "<none>"))
    return "\n".join(out) + "\n"
