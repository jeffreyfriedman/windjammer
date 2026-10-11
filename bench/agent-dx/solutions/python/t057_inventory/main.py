import sys

inv = {}  # sku -> [qty, price]
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "stock":
        sku, qty, price = p[1], int(p[2]), int(p[3])
        if qty <= 0:
            print("error: bad quantity")
        elif sku in inv and inv[sku][1] != price:
            print("error: price mismatch for " + sku)
        else:
            if sku not in inv:
                inv[sku] = [0, price]
            inv[sku][0] += qty
            print("ok %d" % inv[sku][0])
    elif c == "sell":
        sku, qty = p[1], int(p[2])
        if qty <= 0:
            print("error: bad quantity")
        elif sku not in inv:
            print("error: unknown sku " + sku)
        elif inv[sku][0] < qty:
            print("error: insufficient stock for %s (have %d, need %d)" % (sku, inv[sku][0], qty))
        else:
            inv[sku][0] -= qty
            print("sold %d for %d" % (qty, qty * inv[sku][1]))
    elif c == "remove":
        sku = p[1]
        if sku not in inv:
            print("error: unknown sku " + sku)
        elif inv[sku][0] > 0:
            print("error: not empty " + sku)
        else:
            del inv[sku]
            print("removed " + sku)
    elif c == "report":
        if not inv:
            print("(empty)")
            continue
        total = 0
        for sku in sorted(inv):
            q, pr = inv[sku]
            print("%s %d x %d = %d" % (sku, q, pr, q * pr))
            total += q * pr
        print("total value %d" % total)
