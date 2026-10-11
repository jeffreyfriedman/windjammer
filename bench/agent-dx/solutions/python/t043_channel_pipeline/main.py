import sys
import threading
import queue

lines = sys.stdin.read().split("\n")
n = int(lines[0])
stages = lines[1].split() if len(lines) > 1 else []
END = object()


def make_fn(tok):
    name, _, k = tok.partition(":")
    if name == "add":
        return lambda x: [x + int(k)]
    if name == "mul":
        return lambda x: [x * int(k)]
    if name == "gt":
        return lambda x: [x] if x > int(k) else []
    if name == "even":
        return lambda x: [x] if x % 2 == 0 else []
    if name == "odd":
        return lambda x: [x] if x % 2 != 0 else []
    prev = []

    def dedupe(x):
        dup = x in prev
        prev.append(x)
        del prev[:-2]
        return [] if dup else [x]
    return dedupe


def stage(fn, qin, qout):
    while True:
        x = qin.get()
        if x is END:
            qout.put(END)
            return
        for y in fn(x):
            qout.put(y)


def source(qout):
    for i in range(1, n + 1):
        qout.put(i)
    qout.put(END)


qs = [queue.Queue() for _ in range(len(stages) + 1)]
threads = [threading.Thread(target=source, args=(qs[0],))]
for i, tok in enumerate(stages):
    threads.append(threading.Thread(target=stage, args=(make_fn(tok), qs[i], qs[i + 1])))
for t in threads:
    t.start()
items = []
while True:
    x = qs[-1].get()
    if x is END:
        break
    items.append(x)
for t in threads:
    t.join()
print(len(items))
print(" ".join(map(str, items)))
print("sum=%d" % sum(items))
