import sys
import threading

d = sys.stdin.read().split()
w, n = int(d[0]), int(d[1])
vals = [int(x) for x in d[2:2 + n]]
hist = [0] * 10
lock = threading.Lock()
counts = [0] * w


def work(k):
    for i in range(k, n, w):
        with lock:
            hist[vals[i]] += 1
        counts[k] += 1


ts = [threading.Thread(target=work, args=(k,)) for k in range(w)]
for t in ts:
    t.start()
for t in ts:
    t.join()
print("hist:", " ".join(map(str, hist)))
for k in range(w):
    print("worker %d: %d" % (k, counts[k]))
print("checksum: %d" % sum(i * c for i, c in enumerate(hist)))
