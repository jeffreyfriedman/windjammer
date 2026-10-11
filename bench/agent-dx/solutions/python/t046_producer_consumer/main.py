import sys
import threading
import queue

p, k, c = map(int, sys.stdin.read().split()[:3])
q = queue.Queue(4)
got = [[] for _ in range(c)]


def produce(pi):
    for j in range(1, k + 1):
        q.put("p%d-%d" % (pi, j))


def consume(ci):
    while True:
        x = q.get()
        if x is None:
            return
        got[ci].append(x)


prods = [threading.Thread(target=produce, args=(i,)) for i in range(p)]
cons = [threading.Thread(target=consume, args=(i,)) for i in range(c)]
for t in prods + cons:
    t.start()
for t in prods:
    t.join()
for _ in cons:
    q.put(None)
for t in cons:
    t.join()
items = sorted(x for g in got for x in g)
print("items: %d" % len(items))
print("chars: %d" % sum(len(x) for x in items))
for x in items[:3]:
    print("first: " + x)
print("consumers_total: %d" % sum(len(g) for g in got))
