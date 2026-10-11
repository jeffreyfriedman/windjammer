import sys
limit = 1000 + int(sys.argv[1]) * 137
comp = [False] * (limit + 1)
p = 2
while p * p <= limit:
    if not comp[p]:
        for q in range(p * p, limit + 1, p): comp[q] = True
    p += 1
pr = [k for k in range(2, limit + 1) if not comp[k]]
print(limit, len(pr), sum(pr))
