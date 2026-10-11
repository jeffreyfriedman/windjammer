import sys
import re

lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
ok = inv = ovf = emp = 0
pat = re.compile(r"[+-]?[0-9]+")
for t in lines:
    if t == "":
        emp += 1
        print("error empty")
    elif not pat.fullmatch(t):
        inv += 1
        print("error invalid " + t)
    else:
        v = int(t)
        if -2147483648 <= v <= 2147483647:
            ok += 1
            print("ok", v)
        else:
            ovf += 1
            print("error overflow " + t)
print("summary ok=%d invalid=%d overflow=%d empty=%d" % (ok, inv, ovf, emp))
