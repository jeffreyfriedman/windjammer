v = list(range(1000))
total = 0
for r in range(300):
    s = sum(x // 3 for x in (x * r for x in v) if x % 3 == 0)
    total += s % 100000
print(total)
