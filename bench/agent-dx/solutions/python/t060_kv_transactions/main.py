import sys

store = {}
txs = []  # each: dict key -> previous value or None (absent)


def record(k):
    if txs and k not in txs[-1]:
        txs[-1][k] = store.get(k)


for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "set":
        record(p[1])
        store[p[1]] = int(p[2])
    elif c == "get":
        print(store[p[1]] if p[1] in store else "NULL")
    elif c == "delete":
        record(p[1])
        store.pop(p[1], None)
    elif c == "count":
        v = int(p[1])
        print(sum(1 for x in store.values() if x == v))
    elif c == "begin":
        txs.append({})
    elif c == "rollback":
        if not txs:
            print("NO TRANSACTION")
        else:
            for k, old in txs.pop().items():
                if old is None:
                    store.pop(k, None)
                else:
                    store[k] = old
    elif c == "commit":
        if not txs:
            print("NO TRANSACTION")
        else:
            txs.clear()
