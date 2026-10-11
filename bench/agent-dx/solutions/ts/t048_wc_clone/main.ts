import * as fs from "node:fs";
import { Buffer } from "node:buffer";

function count(b: Buffer): [number, number, number] {
  let lines = 0, words = 0, inWord = false;
  for (const c of b) {
    if (c === 10) lines++;
    const ws = c === 32 || c === 9 || c === 10 || c === 13;
    if (!ws && !inWord) words++;
    inWord = !ws;
  }
  return [lines, words, b.length];
}

const args = process.argv.slice(2);
if (args.length === 0) {
  const [l, w, b] = count(fs.readFileSync(0));
  console.log(`${l} ${w} ${b}`);
} else {
  const tot = [0, 0, 0];
  const out: string[] = [];
  let fail = false;
  for (const name of args) {
    let data: Buffer;
    try {
      data = fs.readFileSync(name);
    } catch {
      out.push(`wc: ${name}: No such file`);
      fail = true;
      continue;
    }
    const c = count(data);
    c.forEach((v, i) => (tot[i] += v));
    out.push(`${c[0]} ${c[1]} ${c[2]} ${name}`);
  }
  if (args.length > 1) out.push(`${tot[0]} ${tot[1]} ${tot[2]} total`);
  console.log(out.join("\n"));
  if (fail) process.exitCode = 1;
}
