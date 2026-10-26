m = {}
for i in range(200000):
    k = (i * 7919) % 10007
    m[k] = m.get(k, 0) + 1
best = 0
for j in range(10007):
    v = m.get(j)
    if v is not None and v > best:
        best = v
print(len(m), best)
