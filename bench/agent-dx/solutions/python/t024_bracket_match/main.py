import sys

PAIR = {")": "(", "]": "[", "}": "{"}
lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
for line in lines:
    st = []
    bad = None
    for i, ch in enumerate(line, 1):
        if ch in "([{":
            st.append((ch, i))
        elif ch in PAIR:
            if not st or st[-1][0] != PAIR[ch]:
                bad = i
                break
            st.pop()
    if bad is None and st:
        bad = st[-1][1]
    print("ok" if bad is None else "error %d" % bad)
