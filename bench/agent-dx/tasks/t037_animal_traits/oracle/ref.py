def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["dog Rex", "cat Tom", "duck Don"]), _c(["snake Sid", "spider Sam", "fish Wanda"])]
EDGE = [_c([]), _c(["cat A", "cat B"]), _c(["dog x"] * 5)]
KS = ["dog", "cat", "duck", "snake", "spider", "fish", "cow"]


def rand(rng):
    return _c([f"{rng.choice(KS)} N{rng.randint(0, 99)}" for _ in range(rng.randint(1, 10))])


D = {"dog": ("woof", 4), "cat": ("meow", 4), "duck": ("quack", 2), "snake": ("hiss", 0), "spider": ("...", 8)}


def solve(case):
    out, tot = [], 0
    for ln in case["stdin"].split("\n")[:-1]:
        k, n = ln.split()
        if k not in D:
            out.append(f"unknown kind {k}")
        elif k == "cat":
            out.append(f"{n} the cat purrs softly")
            tot += 4
        else:
            out.append(f"{n} the {k} says {D[k][0]} on {D[k][1]} legs")
            tot += D[k][1]
    out.append(f"legs total={tot}")
    return "\n".join(out) + "\n"
