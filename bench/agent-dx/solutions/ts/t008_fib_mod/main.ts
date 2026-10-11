import * as fs from "node:fs";

type M = [bigint, bigint, bigint, bigint];

function mul(x: M, y: M, m: bigint): M {
  return [
    (x[0] * y[0] + x[1] * y[2]) % m,
    (x[0] * y[1] + x[1] * y[3]) % m,
    (x[2] * y[0] + x[3] * y[2]) % m,
    (x[2] * y[1] + x[3] * y[3]) % m,
  ];
}

function fib(n: bigint, m: bigint): bigint {
  let r: M = [1n % m, 0n, 0n, 1n % m];
  let b: M = [1n % m, 1n % m, 1n % m, 0n];
  while (n > 0n) {
    if (n & 1n) r = mul(r, b, m);
    b = mul(b, b, m);
    n >>= 1n;
  }
  return r[1];
}

const data = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const t = parseInt(data[0], 10);
for (let i = 0; i < t; i++) {
  console.log(String(fib(BigInt(data[1 + 2 * i]), BigInt(data[2 + 2 * i]))));
}
