import sys

LO, HI = -(1 << 31), (1 << 31) - 1


def tdiv(a, b):
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q


def calc(op, a, b):
    if op == "add":
        r = a + b
    elif op == "sub":
        r = a - b
    elif op == "mul":
        r = a * b
    elif op in ("div", "rem"):
        if b == 0:
            return "divzero"
        q = tdiv(a, b)
        r = q if op == "div" else a - q * b
    else:
        if b < 0:
            return "invalid"
        if a in (0, 1):
            r = a if b > 0 else 1
        elif a == -1:
            r = -1 if b % 2 else 1
        elif b > 40:
            return "overflow"
        else:
            r = a ** b
    return str(r) if LO <= r <= HI else "overflow"


for line in sys.stdin.read().split("\n"):
    p = line.split()
    if len(p) == 3:
        print(calc(p[0], int(p[1]), int(p[2])))
