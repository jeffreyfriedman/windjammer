n = 1000000
flags = [True] * n
count = 0
i = 2
while i < n:
    if flags[i]:
        count += 1
        j = i * i
        while j < n:
            flags[j] = False
            j += i
    i += 1
print(count)
