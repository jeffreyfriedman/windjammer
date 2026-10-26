class V2:
    __slots__ = ("x", "y")
    def __init__(self, x, y):
        self.x = x; self.y = y
    def add(self, o):
        return V2(self.x + o.x, self.y + o.y)
    def dot(self, o):
        return self.x * o.x + self.y * o.y
acc = V2(0, 0)
s = 0
for i in range(300000):
    v = V2(i % 13, i % 7)
    acc = acc.add(v)
    s += acc.dot(v) % 1000
print(acc.x, acc.y, s)
