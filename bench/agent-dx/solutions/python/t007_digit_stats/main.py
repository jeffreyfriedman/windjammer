import sys

data = sys.stdin.read().split()
t = int(data[0])
for s in data[1:1 + t]:
    n = int(s)
    d = str(n)
    ds = sum(int(c) for c in d)
    root = 0 if n == 0 else 1 + (n - 1) % 9
    print(ds, root, int(d[::-1]), "yes" if d == d[::-1] else "no")
