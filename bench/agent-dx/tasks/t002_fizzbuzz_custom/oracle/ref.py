def _c(n, a, b):
    return {"stdin": f"{n} {a} {b}\n"}


SAMPLES = [_c(5, 2, 3), _c(15, 3, 5)]
EDGE = [_c(1, 1, 1), _c(1, 2, 3), _c(30, 1, 7), _c(1000, 7, 11), _c(12, 4, 4)]


def rand(rng):
    return _c(rng.randint(1, 200), rng.randint(1, 12), rng.randint(1, 12))


def solve(case):
    n, a, b = map(int, case["stdin"].split())
    out = []
    for i in range(1, n + 1):
        if i % a == 0 and i % b == 0:
            out.append("FizzBuzz")
        elif i % a == 0:
            out.append("Fizz")
        elif i % b == 0:
            out.append("Buzz")
        else:
            out.append(str(i))
    return "\n".join(out) + "\n"
