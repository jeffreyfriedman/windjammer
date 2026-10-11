import sys

D = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if len(p) < 3:
        continue
    v = int(p[0], int(p[1]))
    b = int(p[2])
    if v == 0:
        print("0")
        continue
    s = ""
    while v:
        v, r = divmod(v, b)
        s = D[r] + s
    print(s)
