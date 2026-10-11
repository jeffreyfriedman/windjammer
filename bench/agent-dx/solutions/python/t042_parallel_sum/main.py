import sys
import threading

d = sys.stdin.read().split()
w, n = int(d[0]), int(d[1])
vals = [int(x) for x in d[2:2 + n]]
q, r = divmod(n, w)
chunks = []
pos = 0
for i in range(w):
    ln = q + 1 if i < r else q
    chunks.append(vals[pos:pos + ln])
    pos += ln
sums = [0] * w


def work(i):
    sums[i] = sum(chunks[i])


ts = [threading.Thread(target=work, args=(i,)) for i in range(w)]
for t in ts:
    t.start()
for t in ts:
    t.join()
for i, s in enumerate(sums):
    print("chunk %d: %d" % (i, s))
print("total: %d" % sum(sums))
