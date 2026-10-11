import sys


class Vec:
    def __init__(self, x, y):
        self.x, self.y = x, y

    def __add__(self, o): return Vec(self.x + o.x, self.y + o.y)
    def __sub__(self, o): return Vec(self.x - o.x, self.y - o.y)
    def __mul__(self, o): return self.x * o.x + self.y * o.y
    def __matmul__(self, o): return self.x * o.y - self.y * o.x
    def __eq__(self, o): return self.x == o.x and self.y == o.y
    def scale(self, k): return Vec(self.x * k, self.y * k)
    def __str__(self): return "(%d, %d)" % (self.x, self.y)


for line in sys.stdin.read().split("\n"):
    p = line.split()
    if len(p) != 5:
        continue
    ax, ay, op, bx, by = p[0], p[1], p[2], p[3], p[4]
    a, b = Vec(int(ax), int(ay)), Vec(int(bx), int(by))
    if op == "+":
        print(a + b)
    elif op == "-":
        print(a - b)
    elif op == "*":
        print(a * b)
    elif op == "x":
        print(a @ b)
    elif op == "s":
        print(a.scale(b.x))
    elif op == "==":
        print("true" if a == b else "false")
