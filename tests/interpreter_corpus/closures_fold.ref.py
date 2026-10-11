import sys
k = int(sys.argv[1]); v = list(range(1, 11))
print((10 + k) * k)
print(sum(x * k for x in v if (x * k) % 2 == 0))
acc = k
for x in v: acc = acc * 2 + x
print(acc)
print("[" + ", ".join(str(a * b) for a, b in zip(v, reversed(v))) + "]")
