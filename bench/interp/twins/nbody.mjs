function advance(b, dt) {
  const n = b.length;
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      const dx = b[i].x - b[j].x, dy = b[i].y - b[j].y, dz = b[i].z - b[j].z;
      const d2 = dx * dx + dy * dy + dz * dz;
      const mag = dt / (d2 * Math.sqrt(d2));
      const mj = b[j].m, mi = b[i].m;
      b[i].vx -= dx * mj * mag; b[i].vy -= dy * mj * mag; b[i].vz -= dz * mj * mag;
      b[j].vx += dx * mi * mag; b[j].vy += dy * mi * mag; b[j].vz += dz * mi * mag;
    }
  }
  for (const p of b) { p.x += dt * p.vx; p.y += dt * p.vy; p.z += dt * p.vz; }
}
function energy(b) {
  let e = 0.0;
  for (let i = 0; i < b.length; i++) {
    const p = b[i];
    e += 0.5 * p.m * (p.vx * p.vx + p.vy * p.vy + p.vz * p.vz);
    for (let j = i + 1; j < b.length; j++) {
      const dx = p.x - b[j].x, dy = p.y - b[j].y, dz = p.z - b[j].z;
      e -= p.m * b[j].m / Math.sqrt(dx * dx + dy * dy + dz * dz);
    }
  }
  return e;
}
const B = (x, y, z, vx, vy, vz, m) => ({ x, y, z, vx, vy, vz, m });
const b = [B(0, 0, 0, 0, 0, 0, 39.47), B(4.84, -1.16, -0.10, 0.60, 2.81, -0.02, 0.037),
  B(8.34, 4.12, -0.40, -1.01, 1.82, 0.008, 0.011), B(12.89, -15.11, -0.22, 1.08, 0.86, -0.01, 0.0017),
  B(15.37, -25.91, 0.17, 0.97, 0.59, -0.03, 0.002)];
const before = energy(b);
for (let s = 0; s < 20000; s++) advance(b, 0.01);
const after = energy(b);
console.log(`${Math.trunc(before * 1000000.0)} ${Math.trunc(after * 1000000.0)}`);
