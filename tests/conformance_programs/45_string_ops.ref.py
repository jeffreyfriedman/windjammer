import sys
M = (1 << 64) - 1
s = int(sys.argv[1]); text = ""
for _ in range(300):
    s ^= (s << 13) & M; s ^= s >> 7; s ^= (s << 17) & M
    text += "abcdefghijklmnopqrstuvwxyz"[((s >> 8) % 26)]
up = text.upper(); h = 0xcbf29ce484222325; vow = 0
for c in text:
    if c in "aeiou": vow += 1
    h = ((h ^ ord(c)) * 1099511628211) & M
print(len(text)); print(vow); print(h)
print(up[:12]); print(up[::-1][:12]); print("true" if "ab" in text else "false")
