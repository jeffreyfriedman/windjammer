const t0 = process.hrtime.bigint();
const m = new Map();
for (let i = 0; i < @iters@; i++) {
  const k = (i * 7919) % @keys@;
  m.set(k, (m.get(k) ?? 0) + 1);
}
let best = 0;
for (let j = 0; j < @keys@; j++) { const v = m.get(j); if (v !== undefined && v > best) best = v; }
console.error(`STEADY_NS ${process.hrtime.bigint() - t0}`);
console.log(`${m.size} ${best}`);
