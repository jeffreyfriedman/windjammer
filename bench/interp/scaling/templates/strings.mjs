const t0 = process.hrtime.bigint();
let s = "";
let total = 0;
for (let i = 0; i < @iters@; i++) {
  s += `${i % 10},`;
  if (s.length > 1000) { total += s.length; s = ""; }
}
console.error(`STEADY_NS ${process.hrtime.bigint() - t0}`);
console.log(`${total} ${s.length}`);
