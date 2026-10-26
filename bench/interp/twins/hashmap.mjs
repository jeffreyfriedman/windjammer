const m = new Map();
for (let i = 0; i < 200000; i++) {
  const k = (i * 7919) % 10007;
  m.set(k, (m.get(k) ?? 0) + 1);
}
let best = 0;
for (let j = 0; j < 10007; j++) { const v = m.get(j); if (v !== undefined && v > best) best = v; }
console.log(`${m.size} ${best}`);
