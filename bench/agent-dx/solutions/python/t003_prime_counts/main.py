import sys

data = sys.stdin.read().split()
t = int(data[0])
qs = [int(x) for x in data[1:1 + t]]
m = max(qs + [2])
sieve = bytearray([1]) * (m + 1)
sieve[0] = sieve[1] = 0
for i in range(2, int(m ** 0.5) + 1):
    if sieve[i]:
        sieve[i * i::i] = bytearray(len(range(i * i, m + 1, i)))
cnt = [0] * (m + 1)
last = [0] * (m + 1)
c = 0
lp = 0
for i in range(m + 1):
    if sieve[i]:
        c += 1
        lp = i
    cnt[i] = c
    last[i] = lp
for n in qs:
    print(cnt[n], last[n] if cnt[n] else "none")
