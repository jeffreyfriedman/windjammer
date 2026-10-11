import sys
s = int(sys.argv[1]); limit = 1000 + s * 100
best, best_len = 1, 0
for i in range(1, limit):
    n, st = i, 0
    while n != 1:
        n = n // 2 if n % 2 == 0 else 3 * n + 1
        st += 1
    if st > best_len: best_len, best = st, i
print(limit, best, best_len)
