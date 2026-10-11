import sys
from abc import ABC, abstractmethod


class Shape(ABC):
    name = ""

    @abstractmethod
    def corners(self): ...

    @abstractmethod
    def double_area(self): ...


class Square(Shape):
    def __init__(self, s): self.s = s
    def corners(self): return 4
    def double_area(self): return 2 * self.s * self.s


class Rect(Shape):
    def __init__(self, w, h): self.w, self.h = w, h
    def corners(self): return 4
    def double_area(self): return 2 * self.w * self.h


class RTri(Shape):
    def __init__(self, a, b): self.a, self.b = a, b
    def corners(self): return 3
    def double_area(self): return self.a * self.b


class Trap(Shape):
    def __init__(self, a, b, h): self.a, self.b, self.h = a, b, h
    def corners(self): return 4
    def double_area(self): return (self.a + self.b) * self.h


KINDS = {"square": Square, "rect": Rect, "rtri": RTri, "trap": Trap}
total = 0
best = None
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p or p[0] not in KINDS:
        continue
    s = KINDS[p[0]](*[int(x) for x in p[1:]])
    s.name = p[0]
    d = s.double_area()
    print("%s corners=%d area2=%d" % (s.name, s.corners(), d))
    total += d
    if best is None or d > best[1]:
        best = (s.name, d)
print("total area2=%d" % total)
print("largest=none" if best is None else "largest=%s area2=%d" % best)
