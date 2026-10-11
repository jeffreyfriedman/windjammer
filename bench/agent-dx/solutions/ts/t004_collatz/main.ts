import * as fs from "node:fs";

const data = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const t = parseInt(data[0], 10);
for (let i = 0; i < t; i++) {
  let n = BigInt(data[1 + i]);
  let steps = 0;
  let peak = n;
  while (n !== 1n) {
    n = n % 2n === 0n ? n / 2n : 3n * n + 1n;
    steps++;
    if (n > peak) peak = n;
  }
  console.log(`${steps} ${peak}`);
}
