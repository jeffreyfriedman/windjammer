import sys
seed = int(sys.argv[1]); items = []
for i in range(6):
    items.append("x%d" % i if i == seed else str(i * seed))
err = None; total = 0
for it in items:
    try: total += int(it)
    except ValueError:
        err = "bad:" + it; break
print("err " + err if err else "ok %d" % total)
print("Ok(25)")
