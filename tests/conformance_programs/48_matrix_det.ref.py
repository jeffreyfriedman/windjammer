import sys
from fractions import Fraction
M = (1 << 64) - 1
s = int(sys.argv[1]); m = []
for r in range(5):
    row = []
    for c in range(5):
        s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
        row.append(s % 2001 - 1000)
    m.append(row)
# exact determinant via fraction-based Gaussian elimination (independent of the Laplace expansion)
a = [[Fraction(x) for x in row] for row in m]; n = 5; d = Fraction(1)
for i in range(n):
    p = next((r for r in range(i, n) if a[r][i] != 0), None)
    if p is None: d = Fraction(0); break
    if p != i: a[i], a[p] = a[p], a[i]; d = -d
    d *= a[i][i]
    for r in range(i + 1, n):
        f = a[r][i] / a[i][i]
        for c in range(i, n): a[r][c] -= f * a[i][c]
print(int(d)); print(m[2][3])
