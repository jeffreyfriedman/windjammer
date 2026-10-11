import sys
x = int(sys.argv[1]); total = 0
for i in range(6):
    k = i % 4
    c = 0
    if k == 0:
        c += 1
        if x % 2 == 0: c += 10
    elif k == 1: c += 100
    elif k == 2:
        c += 1000
    else: c += 100
    total += c
    print(i, c)
    x = x * 3 + 1
o = x % 5
print("total %d %s %s" % (total, "true" if o == 0 else "false", "false"))
