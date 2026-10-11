import sys
n = int(sys.argv[1]) * 10000
print(n + sum(k % 7 for k in range(1, n + 1)))
