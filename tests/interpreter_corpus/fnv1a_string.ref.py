import sys
n = int(sys.argv[1])
text = "seed-%d-payload" % n
h = 14695981039346656037
for b in text.encode():
    h ^= b
    h = (h * 1099511628211) & ((1 << 64) - 1)
print(text); print(h); print("%x" % h); print("%016X" % (h >> 8))
