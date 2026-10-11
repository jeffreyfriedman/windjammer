import * as fs from "node:fs";
import { Buffer } from "node:buffer";

const args = process.argv.slice(2);
if (args.length !== 3) {
  console.log("usage: transform INPUT OUTPUT MODE");
  process.exit(2);
}
const [input, output, mode] = args;
const modes = ["upper", "reverse", "sort", "dedupe", "trim", "number"];
let text: string;
try {
  text = fs.readFileSync(input, "latin1");
} catch {
  console.log(`error: cannot read ${input}`);
  process.exit(1);
}
if (!modes.includes(mode)) {
  console.log(`error: unknown mode ${mode}`);
  process.exit(2);
}
let lines = text.split("\n");
if (lines[lines.length - 1] === "") lines.pop();
switch (mode) {
  case "upper": lines = lines.map((l) => l.toUpperCase()); break;
  case "reverse": lines.reverse(); break;
  case "sort": lines.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0)); break;
  case "dedupe": lines = [...new Set(lines)]; break;
  case "trim": lines = lines.map((l) => l.replace(/^ +| +$/g, "")); break;
  case "number": lines = lines.map((l, i) => `${i + 1}:${l}`); break;
}
fs.writeFileSync(output, lines.map((l) => l + "\n").join(""), "latin1");
console.log(`wrote ${lines.length} lines to ${output}`);
fs.writeSync(1, Buffer.from(fs.readFileSync(output, "latin1"), "latin1"));
