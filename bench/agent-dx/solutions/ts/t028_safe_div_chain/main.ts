import * as fs from "node:fs";

const MIN = -(1n << 63n);
const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const out = lines.map((raw) => {
  const nums = raw.trim().split(" / ").map((x) => BigInt(x.trim()));
  let acc = nums[0];
  for (let k = 1; k < nums.length; k++) {
    const d = nums[k];
    if (d === 0n) return `error: division by zero at step ${k}`;
    if (acc === MIN && d === -1n) return `error: overflow at step ${k}`;
    acc /= d;
  }
  return String(acc);
});
if (out.length > 0) console.log(out.join("\n"));
