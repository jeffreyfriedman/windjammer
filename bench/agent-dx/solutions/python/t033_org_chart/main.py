import sys

lines = [l for l in sys.stdin.read().split("\n")]
n = int(lines[0])
emp = {}
for l in lines[1:1 + n]:
    name, mgr, sal = l.split()
    emp[name] = (mgr, int(sal))
for q in lines[1 + n:]:
    q = q.strip()
    if not q:
        continue
    if q not in emp:
        print("none: unknown " + q)
        continue
    m = emp[q][0]
    if m == "-":
        print("none: no manager " + q)
    elif m not in emp:
        print("none: unknown " + m)
    elif emp[m][0] == "-":
        print("none: no skip-level " + m)
    elif emp[m][0] not in emp:
        print("none: unknown " + emp[m][0])
    else:
        print(emp[emp[m][0]][1])
