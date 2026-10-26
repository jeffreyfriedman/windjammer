total = 0
i = 0
while i < 5000000:
    x = i * 3 + 1
    if x % 7 == 0:
        total += x
    else:
        total -= 1
    i += 1
print(total)
