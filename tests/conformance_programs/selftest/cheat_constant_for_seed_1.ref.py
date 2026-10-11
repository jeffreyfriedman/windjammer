import sys
seed = int(sys.argv[1])
print((seed * 7 + 3) * (seed + 11) % 1000003 * 100 + 48)
