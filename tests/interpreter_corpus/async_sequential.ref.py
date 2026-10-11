import sys
s = int(sys.argv[1])
print("created")
print("fetch %d" % s)
print("got %d" % (s * 10))
print("fetch %d" % s)
print("fetch %d" % (s + 1))
print("total %d" % (s * 10 + (s + 1) * 10))
