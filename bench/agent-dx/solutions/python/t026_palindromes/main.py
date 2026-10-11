import sys

lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
for s in lines:
    n = len(s)
    best = ""
    count = 0
    for c in range(2 * n - 1):
        l = c // 2
        r = l + c % 2
        while l >= 0 and r < n and s[l] == s[r]:
            count += 1
            if r - l + 1 > len(best) or (r - l + 1 == len(best) and l < s.find(best)):
                best = s[l:r + 1]
            l -= 1
            r += 1
    # leftmost among maximal: recompute explicitly
    if n:
        m = len(best)
        for i in range(n - m + 1):
            sub = s[i:i + m]
            if sub == sub[::-1]:
                best = sub
                break
        print(m, best, count)
    else:
        print("0 - 0")
