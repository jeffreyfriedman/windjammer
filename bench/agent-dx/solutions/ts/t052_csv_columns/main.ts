import * as fs from "node:fs";

const fail = (msg: string, code: number): never => {
  console.log(msg);
  process.exit(code);
};
const [file, column, op] = process.argv.slice(2);
let text: string;
try {
  text = fs.readFileSync(file, "latin1");
} catch {
  text = fail(`error: cannot read ${file}`, 1);
}
const lines = text.split("\n").map((l) => l.replace(/\r$/, ""));
if (lines[lines.length - 1] === "") lines.pop();
const header = (lines[0] ?? "").split(",");
const idx = header.indexOf(column);
if (idx < 0) fail(`error: no such column ${column}`, 2);
if (!["sum", "min", "max", "count", "mean"].includes(op)) fail(`error: unknown op ${op}`, 2);
const rows = lines.slice(1).map((l) => (l.split(",")[idx] ?? ""));
if (op === "count") {
  console.log(String(rows.length));
} else {
  const nums: bigint[] = [];
  for (let i = 0; i < rows.length; i++) {
    if (!/^[0-9]+$/.test(rows[i])) fail(`error: bad value ${rows[i]} on line ${i + 2}`, 3);
    nums.push(BigInt(rows[i]));
  }
  if (nums.length === 0) console.log(op === "sum" ? "0" : "n/a");
  else {
    const sum = nums.reduce((a, b) => a + b, 0n);
    if (op === "sum") console.log(String(sum));
    else if (op === "mean") console.log(String(sum / BigInt(nums.length)));
    else console.log(String(nums.reduce((a, b) => (op === "min" ? (b < a ? b : a) : b > a ? b : a))));
  }
}
