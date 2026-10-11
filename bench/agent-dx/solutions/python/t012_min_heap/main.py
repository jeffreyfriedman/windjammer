import sys
import heapq

h = []
seq = 0
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    if p[0] == "push":
        heapq.heappush(h, (int(p[1]), seq, p[2]))
        seq += 1
    elif p[0] == "pop":
        if h:
            pr, _, lab = heapq.heappop(h)
            print(pr, lab)
        else:
            print("empty")
    elif p[0] == "peek":
        print("%d %s" % (h[0][0], h[0][2]) if h else "empty")
    elif p[0] == "size":
        print(len(h))
