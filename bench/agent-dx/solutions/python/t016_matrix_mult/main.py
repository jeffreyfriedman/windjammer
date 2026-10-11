import sys

tok = sys.stdin.read().split()
pos = 0


def nxt():
    global pos
    pos += 1
    return int(tok[pos - 1])


def read_matrix():
    r, c = nxt(), nxt()
    return r, c, [[nxt() for _ in range(c)] for _ in range(r)]


for _ in range(nxt()):
    r1, c1, a = read_matrix()
    r2, c2, b = read_matrix()
    if c1 != r2:
        print("dimension mismatch")
    else:
        for i in range(r1):
            print(" ".join(str(sum(a[i][k] * b[k][j] for k in range(c1))) for j in range(c2)))
    print("---")
