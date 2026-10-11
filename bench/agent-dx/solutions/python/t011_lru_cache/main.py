import sys
from collections import OrderedDict

lines = sys.stdin.read().split("\n")
cap = int(lines[0])
c = OrderedDict()  # last = most recent
for line in lines[1:]:
    p = line.split()
    if not p:
        continue
    if p[0] == "put":
        k, v = int(p[1]), int(p[2])
        c[k] = v
        c.move_to_end(k)
        if len(c) > cap:
            ek, _ = c.popitem(last=False)
            print("evict", ek)
    elif p[0] == "get":
        k = int(p[1])
        if k in c:
            c.move_to_end(k)
            print(c[k])
        else:
            print(-1)
    elif p[0] == "keys":
        print(" ".join(str(k) for k in reversed(c)) if c else "<empty>")
