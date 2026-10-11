def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["1 + 2 * 3", "(1 + 2) * 3", "-4 + 10"]), _c(["7 / 2", "-7 / 2", "-7 % 3", "1 / 0", "(1", ""])]
EDGE = [_c(["--5", "2 * -3", "- (2 + 3) * 2", "10 - 3 - 2", "100 / 10 / 5", "2 * (3 + (4 - 1)) % 5", "   42   ", "1 +", "1 2", "a", "()", "0 % 5", "5 % 0", "((((7))))"])]


def _tok(rng):
    return rng.choice(["1", "23", "7", "100", "+", "-", "*", "/", "%", "(", ")", " "])


def _gen(rng, d=0):
    if d > 3 or rng.random() < 0.35:
        return str(rng.randint(0, 50))
    k = rng.random()
    if k < 0.15:
        return "-" + _gen(rng, d + 1)
    if k < 0.3:
        return "(" + _gen(rng, d + 1) + ")"
    return _gen(rng, d + 1) + rng.choice([" + ", " - ", " * ", " / ", " % ", "*", "-"]) + _gen(rng, d + 1)


def rand(rng):
    lines = []
    for _ in range(rng.randint(2, 8)):
        e = _gen(rng)
        if rng.random() < 0.2:
            e = "".join(_tok(rng) for _ in range(rng.randint(1, 6)))
        lines.append(e)
    return _c(lines)


class Err(Exception):
    pass


def _ev(s):
    pos = 0

    def peek():
        nonlocal pos
        while pos < len(s) and s[pos] == " ":
            pos += 1
        return s[pos] if pos < len(s) else None

    def expr():
        nonlocal pos
        v = term()
        while peek() in ("+", "-"):
            op = s[pos]
            pos += 1
            r = term()
            v = v + r if op == "+" else v - r
        return v

    def term():
        nonlocal pos
        v = unary()
        while peek() in ("*", "/", "%"):
            op = s[pos]
            pos += 1
            r = unary()
            if op == "*":
                v = v * r
            else:
                if r == 0:
                    raise Err()
                q = abs(v) // abs(r)
                q = q if (v < 0) == (r < 0) else -q
                v = q if op == "/" else v - q * r
        return v

    def unary():
        nonlocal pos
        c = peek()
        if c == "-":
            pos += 1
            return -unary()
        return atom()

    def atom():
        nonlocal pos
        c = peek()
        if c == "(":
            pos += 1
            v = expr()
            if peek() != ")":
                raise Err()
            pos += 1
            return v
        if c is not None and c.isdigit():
            st = pos
            while pos < len(s) and s[pos].isdigit():
                pos += 1
            return int(s[st:pos])
        raise Err()
    v = expr()
    if peek() is not None:
        raise Err()
    return v


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        try:
            out.append(str(_ev(ln)))
        except Err:
            out.append("error")
    return "\n".join(out) + "\n"
