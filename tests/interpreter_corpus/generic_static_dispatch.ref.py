import sys
seed = int(sys.argv[1])
print("acc %d" % (100 + sum(i * seed for i in range(5))))
print("cnt 4")
print("%d [] 0 []" % (0 + seed))
print("fresh 100")
