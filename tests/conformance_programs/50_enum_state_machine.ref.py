import sys
M = (1 << 64) - 1
s = int(sys.argv[1]); st = ("Idle",); h = 5; dones = 0
def step(st, x):
    k = st[0]
    if k == "Idle": return ("Running", x) if x % 4 == 0 else st
    if k == "Running":
        n = st[1]
        if n > 9000: return ("Done",)
        if x % 5 == 0: return ("Paused", n, x)
        return ("Running", n + x)
    if k == "Paused":
        n, m = st[1], st[2]
        return ("Running", n + m) if x % 3 == 0 else ("Paused", n, m + 1)
    return ("Idle",)
def code(st):
    k = st[0]
    if k == "Idle": return 0
    if k == "Running": return 1 + st[1] % 7
    if k == "Paused": return 10 + (st[1] + st[2]) % 11
    return 99
for _ in range(5000):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    st = step(st, s % 100)
    c = code(st)
    if c == 99: dones += 1
    h = (h * 257 + c) % 1000000007
print(dones, h)
