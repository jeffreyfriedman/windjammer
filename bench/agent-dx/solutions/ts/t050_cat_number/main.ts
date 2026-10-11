import * as fs from "node:fs";
import { Buffer } from "node:buffer";

const args = process.argv.slice(2);
const number = args.includes("-n");
const files = args.filter((a) => a !== "-n");
let out = "";
let counter = 0;
let status = 0;

const emit = (text: string) => {
  if (!number) {
    out += text;
    return;
  }
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  for (const l of lines) out += `${String(++counter).padStart(6)}\t${l}\n`;
};

if (files.length === 0) emit(fs.readFileSync(0, "latin1"));
else {
  for (const f of files) {
    let text: string;
    try {
      text = fs.readFileSync(f, "latin1");
    } catch {
      out += `cat: ${f}: No such file\n`;
      status = 1;
      continue;
    }
    emit(text);
  }
}
fs.writeSync(1, Buffer.from(out, "latin1"));
process.exitCode = status;
