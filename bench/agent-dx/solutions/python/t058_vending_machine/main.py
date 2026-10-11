import sys

PRICES = {"cola": 150, "chips": 100, "gum": 35}
stock = {"cola": 3, "chips": 2, "gum": 5}
COINS = (100, 25, 10, 5)
credit = 0


def change(amount):
    out = []
    for c in COINS:
        while amount >= c:
            out.append(str(c))
            amount -= c
    return " ".join(out) if out else "none"


for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "coin":
        v = int(p[1])
        if v in COINS:
            credit += v
            print("credit %d" % credit)
        else:
            print("rejected %d" % v)
    elif c == "select":
        item = p[1]
        if item not in PRICES:
            print("error: unknown item " + item)
        elif stock[item] == 0:
            print("error: sold out " + item)
        elif credit < PRICES[item]:
            print("error: need %d more" % (PRICES[item] - credit))
        else:
            stock[item] -= 1
            print("dispensed %s change: %s" % (item, change(credit - PRICES[item])))
            credit = 0
    elif c == "refund":
        print("refund: " + change(credit))
        credit = 0
    elif c == "status":
        print("credit=%d cola=%d chips=%d gum=%d" % (credit, stock["cola"], stock["chips"], stock["gum"]))
