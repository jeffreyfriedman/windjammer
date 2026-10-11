import sys
x = 2147483000 + int(sys.argv[1])
print("start %d" % x)
for _ in range(10):
    x += 100
    if x > 2147483647: break
    print(x)
