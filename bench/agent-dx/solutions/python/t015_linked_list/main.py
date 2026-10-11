import sys
from collections import deque

d = deque()
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "push_front":
        d.appendleft(int(p[1]))
    elif c == "push_back":
        d.append(int(p[1]))
    elif c == "pop_front":
        print(d.popleft() if d else "empty")
    elif c == "pop_back":
        print(d.pop() if d else "empty")
    elif c == "remove":
        try:
            d.remove(int(p[1]))
            print("removed")
        except ValueError:
            print("missing")
    elif c == "reverse":
        d.reverse()
    elif c == "print":
        print(" ".join(map(str, d)) if d else "<empty>")
    elif c == "nth":
        k = int(p[1])
        print(d[k] if 0 <= k < len(d) else "none")
