def _c(rows):
    return {"stdin": "".join(f"{n} {a}\n" for n, a in rows)}


SAMPLES = [_c([("fib", 10), ("fib", 10), ("tri", 4), ("collatz", 6)]), _c([("fib", 0), ("fact", 3), ("tri", 0), ("tri", 0)])]
EDGE = [_c([]), _c([("fib", 80), ("fib", 79), ("fib", 80), ("collatz", 1), ("collatz", 97), ("collatz", 100000), ("tri", 100000000)])]


def rand(rng):
    rows = []
    for _ in range(rng.randint(3, 20)):
        n = rng.choice(["fib", "collatz", "tri", "tri", "fib", "oops"])
        a = rng.randint(0, 6) if rng.random() < 0.6 else rng.randint(0, 80) if n == "fib" else rng.randint(1, 100000) if n == "collatz" else rng.randint(0, 10 ** 6)
        if n == "collatz":
            a = max(a, 1)
        rows.append((n, a))
    return _c(rows)


def solve(case):
    seen, out, h, m = set(), [], 0, 0
    for ln in case["stdin"].split("\n")[:-1]:
        n, a = ln.split()
        a = int(a)
        if n == "fib":
            x, y = 0, 1
            for _ in range(a):
                x, y = y, (x + y) % 1000000007
            v = x
        elif n == "collatz":
            v, t = 0, a
            while t != 1:
                t = t // 2 if t % 2 == 0 else 3 * t + 1
                v += 1
        elif n == "tri":
            v = a * (a + 1) // 2
        else:
            out.append("unknown function")
            continue
        if (n, a) in seen:
            h += 1
            out.append(f"{v} hit")
        else:
            m += 1
            seen.add((n, a))
            out.append(f"{v} miss")
    out.append(f"hits={h} misses={m}")
    return "\n".join(out) + "\n"
