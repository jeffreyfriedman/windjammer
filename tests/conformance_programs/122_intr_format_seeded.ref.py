import sys
s = int(sys.argv[1])
for _ in range(30):
    s = (s * 1103515245 + 12345) % 2147483648
    a = (s // 65536) % 100000
    neg = a - 50000
    b = a / 8.0
    w = "w%d" % a
    print("[%s] [%s] [%s] [%s] [%s] [%s]" % (
        format(a, ">8"), format(a, "<8"), format(a, "^8"), format(a, "08"), format(neg, "08"), format(a, "*>9")))
    print("[%s] [%s] [%s] [%s] [%s] [%s]" % (
        format(b, ".2f"), format(b, "10.3f"), format(b, "<10.1f"), format(b, "010.2f"), format(b, "^11.2f"), repr(b)))
    print("[%s] [%s] [%s] [%s]" % (format(a, "x"), format(a, ">8X"), format(a, "b"), format(a, "o")))
    print("[%s] [%s] [%s] [%s] [%s] [%s]" % (
        format(w, ">8"), format(w, "<8"), format(w, "^8"), w[:3], '"' + w + '"', format(w, "_^9")))
