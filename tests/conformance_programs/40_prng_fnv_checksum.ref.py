import sys
M = (1 << 64) - 1
seed = int(sys.argv[1])
s = seed
h = (3421674724 << 32) | 2216829733  
for i in range(1, 20001):
    s ^= (s << 13) & M
    s ^= s >> 7
    s ^= (s << 17) & M
    for k in range(8):
        h = ((h ^ ((s >> (8 * k)) & 255)) * 1099511628211) & M
    if i % 5000 == 0:
        print(i, s, h)
