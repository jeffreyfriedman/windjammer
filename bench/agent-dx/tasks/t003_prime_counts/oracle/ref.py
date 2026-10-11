def _c(ns):
    return {"stdin": f"{len(ns)}\n" + "".join(f"{n}\n" for n in ns)}


SAMPLES = [_c([10]), _c([0, 1, 2, 3, 100])]
EDGE = [_c([2000000]), _c([1999993, 1999994]), _c([4, 5, 6, 7, 8, 9, 10, 11, 12, 13])]


def rand(rng):
    return _c([rng.choice([rng.randint(0, 50), rng.randint(0, 2000000)]) for _ in range(rng.randint(1, 10))])


_N = 2000001
_sieve = bytearray([1]) * _N
_sieve[0] = _sieve[1] = 0
for _i in range(2, int(_N ** 0.5) + 1):
    if _sieve[_i]:
        _sieve[_i * _i::_i] = bytearray(len(_sieve[_i * _i::_i]))


def solve(case):
    ns = list(map(int, case["stdin"].split()))[1:]
    out = []
    for n in ns:
        c, last = 0, None
        # prefix scan is fine for the reference at this size
        c = sum(_sieve[:n + 1])
        p = n
        while p >= 2 and not _sieve[p]:
            p -= 1
        out.append(f"{c} {p}" if p >= 2 else "0 none")
    return "\n".join(out) + "\n"
