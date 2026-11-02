import sys
s = int(sys.argv[1])
v = [(k * 7 + s) % 5 for k in range(s + 5)]
print("len %d" % len(v))
print("sum %d" % sum(v))
swaps = 0
for i in range(len(v)):
    for j in range(i + 1, len(v)):
        if v[j] > v[i]:
            v[i], v[j] = v[j], v[i]
            swaps += 1
print("swaps %d first %d last %d" % (swaps, v[0], v[-1]))
print("positions %d" % len([i + 1 for i in range(len(v)) if v[i] == 2]))
print("count %d last %d" % (len(v), v[-1]))
