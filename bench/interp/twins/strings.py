s = ""
total = 0
for i in range(200000):
    s += f"{i % 10},"
    if len(s) > 1000:
        total += len(s)
        s = ""
print(total, len(s))
