import sys, math
n = int(sys.argv[1]); acc = 0.0
for i in range(1, 51):
    x = float(i * n)
    acc = acc + math.sqrt(x) * 0.5 + x / 7.0
print("%.6f" % acc)
print("%.3f" % (acc / 3.0))
print(int(acc) % 1000)
print("%12.2f|" % acc)
