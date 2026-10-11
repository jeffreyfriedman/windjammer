import sys
M = (1 << 64) - 1
s = int(sys.argv[1])
for _ in range(3):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    print(s)
