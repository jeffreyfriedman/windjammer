import sys
seed = int(sys.argv[1])
def work(i, seed):
    acc = i + seed
    for k in range(50):
        acc = (acc * 31 + k) % 1000003
    return acc
vals = [work(i, seed) for i in range(4)]
for v in vals: print("joined %d" % v)
print("total %d" % sum(vals))
got = sorted(b * 100 + j + seed for b in range(3) for j in range(4))
print("received [" + ", ".join(map(str, got)) + "]")
print("counter %d hits 5" % sum(w + seed for w in range(5)))
