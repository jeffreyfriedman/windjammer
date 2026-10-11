import * as fs from "node:fs";

function gcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) {
    [a, b] = [b, a % b];
  }
  return a;
}

const data = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const t = parseInt(data[0], 10);
for (let i = 0; i < t; i++) {
  const a = BigInt(data[1 + 2 * i]);
  const b = BigInt(data[2 + 2 * i]);
  const g = gcd(a, b);
  console.log(`${g} ${(a / g) * b}`);
}
