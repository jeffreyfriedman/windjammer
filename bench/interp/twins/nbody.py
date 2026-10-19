import math
def advance(b, dt):
    n = len(b)
    for i in range(n):
        for j in range(i + 1, n):
            dx = b[i][0] - b[j][0]; dy = b[i][1] - b[j][1]; dz = b[i][2] - b[j][2]
            d2 = dx * dx + dy * dy + dz * dz
            mag = dt / (d2 * math.sqrt(d2))
            mj = b[j][6]; mi = b[i][6]
            b[i][3] -= dx * mj * mag; b[i][4] -= dy * mj * mag; b[i][5] -= dz * mj * mag
            b[j][3] += dx * mi * mag; b[j][4] += dy * mi * mag; b[j][5] += dz * mi * mag
    for k in range(n):
        b[k][0] += dt * b[k][3]; b[k][1] += dt * b[k][4]; b[k][2] += dt * b[k][5]
def energy(b):
    e = 0.0
    n = len(b)
    for i in range(n):
        x, y, z, vx, vy, vz, m = b[i]
        e += 0.5 * m * (vx * vx + vy * vy + vz * vz)
        for j in range(i + 1, n):
            dx = x - b[j][0]; dy = y - b[j][1]; dz = z - b[j][2]
            e -= m * b[j][6] / math.sqrt(dx * dx + dy * dy + dz * dz)
    return e
b = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 39.47],
     [4.84, -1.16, -0.10, 0.60, 2.81, -0.02, 0.037],
     [8.34, 4.12, -0.40, -1.01, 1.82, 0.008, 0.011],
     [12.89, -15.11, -0.22, 1.08, 0.86, -0.01, 0.0017],
     [15.37, -25.91, 0.17, 0.97, 0.59, -0.03, 0.002]]
before = energy(b)
for _ in range(20000):
    advance(b, 0.01)
after = energy(b)
print(int(before * 1000000.0), int(after * 1000000.0))
