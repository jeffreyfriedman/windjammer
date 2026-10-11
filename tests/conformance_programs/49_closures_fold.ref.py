import sys
M = (1 << 64) - 1
s = int(sys.argv[1]); k = s % 10 + 2; total = 0
for _ in range(1000):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    v = s % 5000
    total = (total + (v * k % 1000003 + k)) % 1000003
print(total)
a = 7
for _ in range(50): a = a * k % 1000003
print(a)
b = 1
for _ in range(12): b = b * 3 + 1
print(b)
