import sys


def is_int(tok):
    body = tok[1:] if tok.startswith("-") else tok
    return body.isascii() and body.isdigit()


def evaluate(line):
    st = []
    for tok in line.split(" "):
        if tok in ("+", "-", "*", "/"):
            if len(st) < 2:
                return "error: underflow"
            b = st.pop()
            a = st.pop()
            if tok == "+":
                st.append(a + b)
            elif tok == "-":
                st.append(a - b)
            elif tok == "*":
                st.append(a * b)
            else:
                if b == 0:
                    return "error: divzero"
                q = abs(a) // abs(b)
                st.append(q if (a < 0) == (b < 0) else -q)
        elif is_int(tok):
            st.append(int(tok))
        else:
            return "error: token"
    if len(st) != 1:
        return "error: leftover"
    return str(st[0])


for line in sys.stdin.read().split("\n"):
    line = line.strip()
    if line:
        print(evaluate(line))
