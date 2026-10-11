def _c(p, k, c):
    return {"stdin": f"{p} {k} {c}\n"}


SAMPLES = [_c(2, 3, 2), _c(1, 0, 1)]
EDGE = [_c(6, 200, 4), _c(3, 1, 4), _c(6, 10, 1)]


def rand(rng):
    return _c(rng.randint(1, 6), rng.randint(0, 40), rng.randint(1, 4))


def solve(case):
    p, k, c = map(int, case["stdin"].split())
    items = [f"p{a}-{j}" for a in range(p) for j in range(1, k + 1)]
    out = [f"items: {len(items)}", f"chars: {sum(map(len, items))}"]
    out += [f"first: {x}" for x in sorted(items)[:3]]
    out.append(f"consumers_total: {len(items)}")
    return "\n".join(out) + "\n"
