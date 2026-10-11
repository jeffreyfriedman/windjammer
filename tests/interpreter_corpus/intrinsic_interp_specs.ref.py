import sys
s = int(sys.argv[1]); n = s * 37 + 5; f = n / 7.0; name = "w-bat"
print("n=%d f=%.2f f3=%.3f q=\"%s\"" % (n, f, f, name))
print("r=[%6d] l=[%-6d] z=%04d hex=%x c=[%s]" % (n, n, n, n, name.center(9)))
print("sum=%d sq=%d half=%.1f" % (n + 1, n * n, n / 2.0))
print("literal {braces} and %d kept" % n)
print("v=[%d, %d, %d] len=3" % (n, n + 1, n + 2))
print("o=Some(%d)" % n)
print("ab%d" % n)
print()
print()
print("end")
