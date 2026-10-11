import sys
state = int(sys.argv[1]); v = []
for _ in range(200):
    state = (state * 48271 + 11) % 1000003
    v.append(state - 500000)
v.sort()
print(v[0], v[199])
print(sum((i + 1) * x for i, x in enumerate(v)))
d = []
for x in v:
    if not d or d[-1] != x: d.append(x)
print(len(d))
