import itertools


def _c(rows):
    return {"stdin": "".join(f"{k} {m} {n}\n" for k, m, n in rows)}


SAMPLES = [_c([("fib", "take", 10), ("squares", "below", 50)]), _c([("primes", "take", 8), ("evens", "sum", 5), ("triangular", "take", 0)])]
EDGE = [_c([("fib", "take", 1), ("fib", "below", 0), ("fib", "below", 100000), ("primes", "below", 2), ("primes", "below", 100000), ("fib", "take", 60), ("triangular", "sum", 60),
            ("squares", "sum", 60), ("evens", "below", 1), ("primes", "sum", 60)])]


def seq(k):
    if k == "fib":
        a, b = 0, 1
        while True:
            yield a
            a, b = b, a + b
    elif k == "squares":
        for i in itertools.count(1):
            yield i * i
    elif k == "primes":
        for n in itertools.count(2):
            if all(n % d for d in range(2, int(n ** 0.5) + 1)):
                yield n
    elif k == "evens":
        for i in itertools.count(0):
            yield 2 * i
    else:
        for i in itertools.count(1):
            yield i * (i + 1) // 2


def rand(rng):
    rows = []
    for _ in range(rng.randint(2, 8)):
        m = rng.choice(["take", "below", "sum"])
        rows.append((rng.choice(["fib", "squares", "primes", "evens", "triangular"]), m, rng.randint(0, 60) if m != "below" else rng.choice([rng.randint(0, 200), rng.randint(0, 100000)])))
    return _c(rows)


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        k, m, n = ln.split()
        n = int(n)
        if m == "take":
            out.append(" ".join(map(str, itertools.islice(seq(k), n))))
        elif m == "below":
            out.append(" ".join(map(str, itertools.takewhile(lambda x: x < n, seq(k)))))
        else:
            out.append(str(sum(itertools.islice(seq(k), n))))
    return "\n".join(out) + "\n"
