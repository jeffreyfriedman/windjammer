import sys
import re
from collections import Counter

text = sys.stdin.buffer.read().decode("latin-1")
first, _, rest = text.partition("\n")
k = int(first.strip())
words = re.findall(r"[A-Za-z0-9]+", rest)
cnt = Counter(w.lower() for w in words)
items = sorted(cnt.items(), key=lambda kv: (-kv[1], kv[0]))
for w, c in items[:k]:
    print(w, c)
