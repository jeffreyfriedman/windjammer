import sys

d = sys.stdin.read().split()
n = int(d[0]) if d else 0
iv = sorted((int(d[1 + 2 * i]), int(d[2 + 2 * i])) for i in range(n))
res = []
for a, b in iv:
    if res and a <= res[-1][1]:
        res[-1][1] = max(res[-1][1], b)
    else:
        res.append([a, b])
for a, b in res:
    print(a, b)
