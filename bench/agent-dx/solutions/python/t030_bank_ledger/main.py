import sys

acc = {}
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "balance":
        print(acc[p[1]] if p[1] in acc else "error: unknown account " + p[1])
        continue
    if c == "open":
        name, amt = p[1], int(p[2])
        if amt < 0:
            print("error: invalid amount")
        elif name in acc:
            print("error: duplicate account")
        else:
            acc[name] = amt
            print("ok")
    elif c in ("deposit", "withdraw"):
        name, amt = p[1], int(p[2])
        if amt <= 0:
            print("error: invalid amount")
        elif name not in acc:
            print("error: unknown account " + name)
        elif c == "withdraw" and acc[name] < amt:
            print("error: insufficient funds")
        else:
            acc[name] += amt if c == "deposit" else -amt
            print("ok")
    elif c == "transfer":
        a, b, amt = p[1], p[2], int(p[3])
        if amt <= 0:
            print("error: invalid amount")
        elif a not in acc:
            print("error: unknown account " + a)
        elif b not in acc:
            print("error: unknown account " + b)
        elif a == b:
            print("error: same account")
        elif acc[a] < amt:
            print("error: insufficient funds")
        else:
            acc[a] -= amt
            acc[b] += amt
            print("ok")
for name in sorted(acc):
    print(name, acc[name])
