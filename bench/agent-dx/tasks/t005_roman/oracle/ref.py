_T = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
      (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]


def to_roman(n):
    s = ""
    for v, r in _T:
        while n >= v:
            s += r
            n -= v
    return s


def _c(items):
    return {"stdin": "".join(f"{x}\n" for x in items)}


SAMPLES = [_c([4, "XIV"]), _c([1994, "MMXXIV", 3999])]
EDGE = [_c([1, "I", 3999, "MMMCMXCIX", 3888, "MMMDCCCLXXXVIII"]), _c([49, 40, 90, 400, 900])]


def rand(rng):
    items = []
    for _ in range(rng.randint(1, 10)):
        n = rng.randint(1, 3999)
        items.append(n if rng.random() < 0.5 else to_roman(n))
    return _c(items)


def solve(case):
    out = []
    val = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}
    for ln in case["stdin"].split("\n"):
        ln = ln.strip()
        if not ln:
            continue
        if ln.isdigit():
            out.append(to_roman(int(ln)))
        else:
            t = 0
            for i, ch in enumerate(ln):
                v = val[ch]
                if i + 1 < len(ln) and val[ln[i + 1]] > v:
                    t -= v
                else:
                    t += v
            out.append(str(t))
    return "\n".join(out) + "\n"
