import sys

M = 1 << 64


class Err(Exception):
    pass


def tokenize(s):
    toks = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == " ":
            i += 1
        elif c.isascii() and c.isdigit():
            j = i
            while j < len(s) and s[j].isascii() and s[j].isdigit():
                j += 1
            toks.append(int(s[i:j]))
            i = j
        elif c in "+-*/%()":
            toks.append(c)
            i += 1
        else:
            raise Err()
    return toks


def tdiv(a, b):
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q


class P:
    def __init__(self, toks):
        self.t = toks
        self.i = 0

    def peek(self):
        return self.t[self.i] if self.i < len(self.t) else None

    def expr(self):
        v = self.term()
        while self.peek() in ("+", "-"):
            op = self.t[self.i]
            self.i += 1
            r = self.term()
            v = v + r if op == "+" else v - r
        return v

    def term(self):
        v = self.unary()
        while self.peek() in ("*", "/", "%"):
            op = self.t[self.i]
            self.i += 1
            r = self.unary()
            if op == "*":
                v = v * r
            else:
                if r == 0:
                    raise Err()
                q = tdiv(v, r)
                v = q if op == "/" else v - q * r
        return v

    def unary(self):
        p = self.peek()
        if p == "-":
            self.i += 1
            return -self.unary()
        if p == "(":
            self.i += 1
            v = self.expr()
            if self.peek() != ")":
                raise Err()
            self.i += 1
            return v
        if isinstance(p, int):
            self.i += 1
            return p
        raise Err()


def evaluate(s):
    try:
        p = P(tokenize(s))
        v = p.expr()
        if p.i != len(p.t):
            return "error"
        return str(v)
    except (Err, RecursionError):
        return "error"


lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
for line in lines:
    print(evaluate(line))
