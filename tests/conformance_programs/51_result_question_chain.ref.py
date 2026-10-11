import sys
M = (1 << 64) - 1
s = int(sys.argv[1]); ok = small = big = odd = sum_ = 0
def rng(v):
    if v < 10: return "too small"
    if v > 990: return "too big"
def pipeline(v):
    e = rng(v)
    if e: return e
    if v % 2: return "odd"
    b = v // 2
    e = rng(b + 3)
    if e: return e
    return (b + 3) * 2
for _ in range(4000):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    r = pipeline(s % 1000)
    if isinstance(r, int): ok += 1; sum_ += r
    elif r == "too small": small += 1
    elif r == "too big": big += 1
    else: odd += 1
print(ok, small, big, odd, sum_)
