import * as fs from "node:fs";

const [n, a, b] = fs.readFileSync(0, "utf8").trim().split(/\s+/).map(Number);
const out: string[] = [];
for (let i = 1; i <= n; i++) {
  const x = i % a === 0, y = i % b === 0;
  out.push(x && y ? "FizzBuzz" : x ? "Fizz" : y ? "Buzz" : String(i));
}
if (out.length > 0) console.log(out.join("\n"));
