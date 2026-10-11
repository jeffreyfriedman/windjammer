import sys

lines = sys.stdin.read().split("\n")
n = int(lines[0])
par = list(range(n))
sz = [1] * n


def find(x):
    while par[x] != x:
        par[x] = par[par[x]]
        x = par[x]
    return x


comps = n
for line in lines[1:]:
    p = line.split()
    if not p:
        continue
    if p[0] == "union":
        a, b = find(int(p[1])), find(int(p[2]))
        if a == b:
            print("same")
        else:
            if sz[a] < sz[b]:
                a, b = b, a
            par[b] = a
            sz[a] += sz[b]
            comps -= 1
            print("merged")
    elif p[0] == "find":
        print("yes" if find(int(p[1])) == find(int(p[2])) else "no")
    elif p[0] == "size":
        print(sz[find(int(p[1]))])
print("components", comps)
