const t0 = process.hrtime.bigint();
const n = @n@;
const flags = new Array(n).fill(true);
let count = 0;
for (let i = 2; i < n; i++) {
  if (flags[i]) {
    count++;
    for (let j = i * i; j < n; j += i) flags[j] = false;
  }
}
console.error(`STEADY_NS ${process.hrtime.bigint() - t0}`);
console.log(String(count));
