const t0 = process.hrtime.bigint();
const v = [];
for (let k = 0; k < @len@; k++) v.push(1.5);
const get = (v, i) => v[i];
let s = 0.0;
for (let c = 0; c < @calls@; c++) s += get(v, c % @len@);
console.error(`STEADY_NS ${process.hrtime.bigint() - t0}`);
console.log(String(Math.trunc(s * 2.0)));
