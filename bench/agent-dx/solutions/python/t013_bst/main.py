import sys


class Node:
    def __init__(self, k):
        self.k = k
        self.l = None
        self.r = None


def insert(n, k):
    if n is None:
        return Node(k), True
    if k == n.k:
        return n, False
    if k < n.k:
        n.l, ok = insert(n.l, k)
    else:
        n.r, ok = insert(n.r, k)
    return n, ok


def delete(n, k):
    if n is None:
        return None, False
    if k < n.k:
        n.l, ok = delete(n.l, k)
        return n, ok
    if k > n.k:
        n.r, ok = delete(n.r, k)
        return n, ok
    if n.l is None:
        return n.r, True
    if n.r is None:
        return n.l, True
    s = n.r
    while s.l:
        s = s.l
    n.k = s.k
    n.r, _ = delete(n.r, s.k)
    return n, True


def inorder(n, out):
    while n is not None:
        inorder(n.l, out)
        out.append(n.k)
        n = n.r


def height(n):
    return 0 if n is None else 1 + max(height(n.l), height(n.r))


sys.setrecursionlimit(10000)
root = None
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    c = p[0]
    if c == "insert":
        root, ok = insert(root, int(p[1]))
        print("ok" if ok else "dup")
    elif c == "contains":
        k = int(p[1])
        n = root
        while n and n.k != k:
            n = n.l if k < n.k else n.r
        print("yes" if n else "no")
    elif c == "delete":
        root, ok = delete(root, int(p[1]))
        print("ok" if ok else "missing")
    elif c == "inorder":
        out = []
        inorder(root, out)
        print(" ".join(map(str, out)) if out else "<empty>")
    elif c == "height":
        print(height(root))
    elif c in ("min", "max"):
        n = root
        if n is None:
            print("empty")
        else:
            while (n.l if c == "min" else n.r):
                n = n.l if c == "min" else n.r
            print(n.k)
