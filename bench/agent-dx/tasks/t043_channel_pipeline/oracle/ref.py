def _c(n, stages):
    return {"stdin": f"{n}\n{' '.join(stages)}\n"}


SAMPLES = [_c(10, ["mul:2", "gt:5"]), _c(5, [])]
EDGE = [_c(0, ["add:1"]), _c(20, ["even", "mul:-1", "add:100", "odd"]), _c(2000, ["mul:3", "gt:100", "even", "add:-7"]), _c(12, ["mul:0", "dedupe3"]), _c(12, ["add:-3", "mul:-1", "even", "dedupe3"]),
        _c(30, ["gt:1000"])]


def rand(rng):
    toks = ["add:{}", "mul:{}", "gt:{}", "even", "odd", "dedupe3"]
    st = []
    for _ in range(rng.randint(0, 5)):
        t = rng.choice(toks)
        st.append(t.format(rng.randint(-5, 25)))
    return _c(rng.randint(0, 60), st)


def solve(case):
    ls = case["stdin"].split("\n")
    n = int(ls[0])
    items = list(range(1, n + 1))
    for tok in ls[1].split():
        if tok.startswith("add:"):
            items = [x + int(tok[4:]) for x in items]
        elif tok.startswith("mul:"):
            items = [x * int(tok[4:]) for x in items]
        elif tok.startswith("gt:"):
            items = [x for x in items if x > int(tok[3:])]
        elif tok == "even":
            items = [x for x in items if x % 2 == 0]
        elif tok == "odd":
            items = [x for x in items if x % 2 != 0]
        else:
            o, seen = [], []
            for x in items:
                if x not in seen[-2:]:
                    o.append(x)
                seen.append(x)
            items = o
    return f"{len(items)}\n{' '.join(map(str, items))}\nsum={sum(items)}\n"
