const t0 = process.hrtime.bigint();
const v = Array.from({ length: @len@ }, (_, k) => k);
let total = 0;
for (let r = 0; r < @rounds@; r++) {
  const s = v.map((x) => x * r).filter((x) => x % 3 === 0).map((x) => x / 3).reduce((a, b) => a + b, 0);
  total += s % 100000;
}
console.error(`STEADY_NS ${process.hrtime.bigint() - t0}`);
console.log(String(total));
