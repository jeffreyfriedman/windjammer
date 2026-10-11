import sys
from collections import deque

lines = sys.stdin.read().split("\n")
cap = int(lines[0])
q = deque()
for line in lines[1:]:
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "push":
        if len(q) >= cap:
            print("full")
        else:
            q.append(int(p[1]))
            print("ok")
    elif c == "pop":
        print(q.popleft() if q else "empty")
    elif c == "peek":
        print(q[0] if q else "empty")
    elif c == "size":
        print(len(q))
    elif c == "dump":
        print(" ".join(map(str, q)) if q else "<empty>")
