const v = Array.from({ length: 1000 }, (_, k) => k);
let total = 0;
for (let r = 0; r < 300; r++) {
  const s = v.map((x) => x * r).filter((x) => x % 3 === 0).map((x) => x / 3).reduce((a, b) => a + b, 0);
  total += s % 100000;
}
console.log(String(total));
