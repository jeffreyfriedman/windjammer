import sys

VALS = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]
SYM = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}


def to_roman(n):
    r = ""
    for v, s in VALS:
        while n >= v:
            r += s
            n -= v
    return r


def from_roman(s):
    total = 0
    for i, ch in enumerate(s):
        v = SYM[ch]
        if i + 1 < len(s) and SYM[s[i + 1]] > v:
            total -= v
        else:
            total += v
    return total


for line in sys.stdin.read().split("\n"):
    line = line.strip()
    if not line:
        continue
    if line.isdigit():
        print(to_roman(int(line)))
    else:
        print(from_roman(line))
