import sys
s = int(sys.argv[1]); acc = 0; log = []
for i in range(200):
    s = (s * 1103515245 + 12345) % 2147483648
    v = s % 1000
    acc = (acc * 31 + v) % 1000000007
    if i % 40 == 0:
        log.append(acc)
        print("%s %s %s" % (format(i, ">4"), format(acc, "08"), format(acc / 8.0, ".3f")))
print("log %d [%s]" % (len(log), ", ".join(str(x) for x in log)))
print("done %s" % format(acc, "x"))
