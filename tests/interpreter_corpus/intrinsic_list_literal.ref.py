import sys
n = int(sys.argv[1])
xs = [n, n*2, n*3, n*4, 100]
print("%s %d" % (xs, len(xs)))
b = [n, 1, n+5, n+6]
print("%s %d" % (b, len(b)))
f = [n, n+1, n+2]
print("%s 3 %d" % (f, f[2]))
print([i*i for i in range(n)])
print("total=%d" % sum(xs))
print([n, n])
