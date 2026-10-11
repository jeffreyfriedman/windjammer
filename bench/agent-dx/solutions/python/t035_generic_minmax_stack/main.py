import sys


class MinMaxStack:
    def __init__(self):
        self.items = []  # (value, min, max)

    def push(self, x):
        if self.items:
            _, lo, hi = self.items[-1]
            self.items.append((x, min(lo, x), max(hi, x)))
        else:
            self.items.append((x, x, x))

    def pop(self):
        return self.items.pop()[0] if self.items else None

    def min(self):
        return self.items[-1][1] if self.items else None

    def max(self):
        return self.items[-1][2] if self.items else None


lines = sys.stdin.read().split("\n")
kind = lines[0].strip()
st = MinMaxStack()


def show(v):
    print("empty" if v is None else v)


for line in lines[1:]:
    if line == "push" or line.startswith("push "):
        arg = line[5:]
        st.push(int(arg) if kind == "int" else arg)
    elif line == "pop":
        show(st.pop())
    elif line == "min":
        show(st.min())
    elif line == "max":
        show(st.max())
    elif line == "size":
        print(len(st.items))
