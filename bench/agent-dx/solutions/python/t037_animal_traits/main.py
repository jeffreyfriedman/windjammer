import sys


class Animal:
    kind = ""
    sound = ""
    legs = 0

    def __init__(self, name):
        self.name = name

    def describe(self):
        return "%s the %s says %s on %d legs" % (self.name, self.kind, self.sound, self.legs)


class Dog(Animal):
    kind, sound, legs = "dog", "woof", 4


class Cat(Animal):
    kind, sound, legs = "cat", "meow", 4

    def describe(self):
        return "%s the cat purrs softly" % self.name


class Duck(Animal):
    kind, sound, legs = "duck", "quack", 2


class Snake(Animal):
    kind, sound, legs = "snake", "hiss", 0


class Spider(Animal):
    kind, sound, legs = "spider", "...", 8


KINDS = {c.kind: c for c in (Dog, Cat, Duck, Snake, Spider)}
total = 0
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    if p[0] in KINDS:
        a = KINDS[p[0]](p[1] if len(p) > 1 else "")
        print(a.describe())
        total += a.legs
    else:
        print("unknown kind " + p[0])
print("legs total=%d" % total)
