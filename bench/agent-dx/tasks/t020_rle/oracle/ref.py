import itertools


def enc(s):
    return "".join(f"{len(list(g))}{k}" for k, g in itertools.groupby(s))


def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["E aaabcc", "D 3a1b2c"]), _c(["E z", "D 1z", "E ", "D "])]
EDGE = [_c(["E " + "a" * 12 + "b" * 100, "D 12a100b", "D 10z1y"]), _c(["E abcabc", "E aabbaa"])]


def rand(rng):
    lines = []
    for _ in range(rng.randint(1, 8)):
        s = "".join(rng.choice("abc") * rng.randint(1, 15) for _ in range(rng.randint(0, 6)))
        lines.append("E " + s if rng.random() < 0.5 else "D " + enc(s))
    return _c(lines)


def solve(case):
    import re
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        mode, _, s = ln.partition(" ")
        if mode == "E":
            out.append(enc(s))
        else:
            out.append("".join(k * int(n) for n, k in re.findall(r"(\d+)([a-z])", s)))
    return "\n".join(out) + "\n"
