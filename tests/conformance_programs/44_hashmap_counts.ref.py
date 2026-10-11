import sys
M = (1 << 64) - 1
s = int(sys.argv[1]); counts = {}
for _ in range(5000):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    k = s % 97
    counts[k] = counts.get(k, 0) + 1
keys = sorted(counts)
h = 11; best = 0
for k in keys:
    c = counts[k]
    h = (h * 1000003 + k * 7919 + c) % 998244353
    best = max(best, c)
print("distinct", len(keys)); print("best", best)
print("first", keys[0], "last", keys[-1]); print("hash", h)
