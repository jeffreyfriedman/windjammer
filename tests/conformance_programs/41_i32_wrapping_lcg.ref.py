import sys
def wrap32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >= (1 << 31) else v
x = int(sys.argv[1]); acc = 0
for i in range(1, 10001):
    x = wrap32(wrap32(x * 1103515245) + 12345)
    acc += x
    if i % 2000 == 0:
        print(i, x, acc)
