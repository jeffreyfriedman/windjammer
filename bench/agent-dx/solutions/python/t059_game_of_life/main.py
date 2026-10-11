import sys

lines = sys.stdin.read().split("\n")
w, h, g = map(int, lines[0].split())
grid = [[ch == "#" for ch in lines[1 + y][:w]] for y in range(h)]
OFF = [(dx, dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dx, dy) != (0, 0)]
for _ in range(g):
    new = []
    for y in range(h):
        row = []
        for x in range(w):
            n = sum(grid[(y + dy) % h][(x + dx) % w] for dx, dy in OFF)
            row.append(n == 3 or (grid[y][x] and n == 2))
        new.append(row)
    grid = new
for row in grid:
    print("".join("#" if c else "." for c in row))
print("alive: %d" % sum(sum(r) for r in grid))
