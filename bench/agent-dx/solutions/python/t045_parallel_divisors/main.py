import sys
from concurrent.futures import ThreadPoolExecutor


def divisors(n):
    cnt = 0
    i = 1
    while i * i <= n:
        if n % i == 0:
            cnt += 1 if i * i == n else 2
        i += 1
    return cnt


d = sys.stdin.read().split()
w, n = int(d[0]), int(d[1])
vals = [int(x) for x in d[2:2 + n]]
with ThreadPoolExecutor(max_workers=w) as ex:
    res = list(ex.map(divisors, vals))
for v, c in zip(vals, res):
    print(v, c)
