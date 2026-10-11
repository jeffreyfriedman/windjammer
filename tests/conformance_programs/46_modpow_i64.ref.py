import sys
seed = int(sys.argv[1]); m = 1000000007; acc = 0
for i in range(1, 201):
    v = pow(seed * 7919 + i, 1000000005 - i, m)
    acc = (acc + v * i) % m
    if i % 50 == 0:
        print(i, v, acc)
