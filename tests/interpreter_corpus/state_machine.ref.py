import sys
i = int(sys.argv[1]); s = ("idle",)
for _ in range(8):
    k = s[0]
    if k == "idle": s = ("run", i)
    elif k == "run":
        n = s[1]
        s = ("done", n) if n > 50 else ("run", n * 2 + i % 3)
    print({"idle": "idle", "run": "run:%d" % (s[1] if s[0]=="run" else 0), "done": "done:%d" % (s[1] if s[0]=="done" else 0)}[s[0]])
    i = i * 7 + 3
