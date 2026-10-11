import sys
M = (1 << 64) - 1
s = int(sys.argv[1]); data = []
for _ in range(400):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    data.append(s % 100000 - 50000)
def cs(v):
    h = 17
    for x in v:
        h = (h * 31 + x)
        # Rust '%' truncates toward zero
        r = abs(h) % 1000000007
        h = -r if h < 0 else r
    return h
print("unsorted", cs(data))
srt = sorted(data)
print("merge", cs(srt))
print("insertion", cs(srt))
print("min", srt[0], "max", srt[-1])
print("median", srt[200])
