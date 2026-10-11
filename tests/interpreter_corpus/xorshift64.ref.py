import sys
M = (1 << 64) - 1
x = int(sys.argv[1])
for _ in range(6):
    x ^= (x << 13) & M
    x ^= x >> 7
    x ^= (x << 17) & M
    print(x)
