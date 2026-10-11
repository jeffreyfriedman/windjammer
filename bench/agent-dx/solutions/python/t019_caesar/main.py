import sys

text = sys.stdin.buffer.read().decode("latin-1")
first, nl, rest = text.partition("\n")
mode, n = first.split()
n = int(n)
if mode == "dec":
    n = -n
n %= 26
out = []
for ch in rest:
    if "a" <= ch <= "z":
        out.append(chr((ord(ch) - 97 + n) % 26 + 97))
    elif "A" <= ch <= "Z":
        out.append(chr((ord(ch) - 65 + n) % 26 + 65))
    else:
        out.append(ch)
sys.stdout.buffer.write("".join(out).encode("latin-1"))
