def _c(rows):
    return {"stdin": f"{len(rows)}\n" + "".join(f"{n} {m}\n" for n, m in rows)}


SAMPLES = [_c([(10, 1000)]), _c([(0, 5), (1, 5), (50, 1000000007)])]
EDGE = [_c([(10 ** 18, 10 ** 9), (10 ** 18, 1), (10 ** 18 - 1, 999999937), (90, 10 ** 9)])]


def rand(rng):
    return _c([(rng.choice([rng.randint(0, 100), rng.randint(0, 10 ** 18)]), rng.randint(1, 10 ** 9))
               for _ in range(rng.randint(1, 8))])


def solve(case):
    xs = list(map(int, case["stdin"].split()))[1:]
    out = []
    for i in range(0, len(xs), 2):
        n, m = xs[i], xs[i + 1]
        a, b = 0, 1  # matrix power by fast doubling
        for bit in bin(n)[2:]:
            c = a * ((2 * b - a) % m) % m
            d = (a * a + b * b) % m
            a, b = (d, (c + d) % m) if bit == "1" else (c, d)
        out.append(str(a % m))
    return "\n".join(out) + "\n"
