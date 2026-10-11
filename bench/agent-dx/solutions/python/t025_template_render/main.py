import sys
import re

text = sys.stdin.read()
first, _, rest = text.partition("\n")
k = int(first.strip())
vals = {}
for _ in range(k):
    line, _, rest = rest.partition("\n")
    key, _, v = line.partition("=")
    vals[key] = v
pat = re.compile(r"\{\{ *([A-Za-z0-9_]+) *\}\}")
sys.stdout.write(pat.sub(lambda m: vals.get(m.group(1), "<missing:%s>" % m.group(1)), rest))
