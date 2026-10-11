import sys
M = (1 << 64) - 1
def rem(a, m):
    r = abs(a) % m
    return -r if a < 0 else r
s = int(sys.argv[1])
a = [1, 2]; b = list(a); pos = list(a); hist = []
snap = (pos[0], pos[1], len(hist))
h = 7
for _ in range(3000):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    d = s % 1000
    if d % 3 == 0:
        a[0] += d; a[1] -= d
    elif d % 3 == 1:
        b[0] += d
        c = list(b)
        b[1] += c[0]
    else:
        pos[0] += d; hist.append(d)
        cp = list(pos)
        pos[1] = cp[1] + 1
    h = rem(h * 131 + a[0] + b[1] * 3 + pos[0] + pos[1], 1000000007)
print(a[0], a[1])
print(b[0], b[1])
print(pos[0], pos[1], len(hist))
print(snap[0], snap[1], snap[2])
print(h)
