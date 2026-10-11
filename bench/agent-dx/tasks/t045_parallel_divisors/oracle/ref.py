def _c(w, xs):
    return {"stdin": f"{w} {len(xs)}\n" + " ".join(map(str, xs)) + "\n"}


SAMPLES = [_c(2, [12, 7, 1]), _c(1, [100])]
EDGE = [_c(4, [1, 2, 3, 4, 999999937, 1000000000, 735134400, 999999999, 1000000000, 4, 9, 16]), _c(8, [1])]


def rand(rng):
    return _c(rng.randint(1, 8), [rng.choice([rng.randint(1, 100), rng.randint(1, 10 ** 9), rng.randint(1, 10 ** 6)]) for _ in range(rng.randint(1, 20))])


def divs(n):
    c, i = 0, 1
    while i * i <= n:
        if n % i == 0:
            c += 1 if i * i == n else 2
        i += 1
    return c


def solve(case):
    t = list(map(int, case["stdin"].split()))
    return "".join(f"{n} {divs(n)}\n" for n in t[2:2 + t[1]])
