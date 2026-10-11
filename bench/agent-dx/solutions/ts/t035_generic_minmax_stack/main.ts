import * as fs from "node:fs";
import { Buffer } from "node:buffer";

class MinMaxStack<T> {
  private items: T[] = [];
  private mins: T[] = [];
  private maxs: T[] = [];
  private cmp: (a: T, b: T) => number;
  constructor(cmp: (a: T, b: T) => number) {
    this.cmp = cmp;
  }
  push(x: T): void {
    const n = this.items.length;
    this.items.push(x);
    this.mins.push(n > 0 && this.cmp(this.mins[n - 1], x) <= 0 ? this.mins[n - 1] : x);
    this.maxs.push(n > 0 && this.cmp(this.maxs[n - 1], x) >= 0 ? this.maxs[n - 1] : x);
  }
  pop(): T | undefined {
    this.mins.pop();
    this.maxs.pop();
    return this.items.pop();
  }
  min(): T | undefined { return this.mins[this.mins.length - 1]; }
  max(): T | undefined { return this.maxs[this.maxs.length - 1]; }
  size(): number { return this.items.length; }
}

const lines = fs.readFileSync(0, "latin1").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const kind = lines[0].trim();
const show = (v: unknown) => (v === undefined ? "empty" : String(v));
const run = <T>(st: MinMaxStack<T>, parse: (s: string) => T) => {
  const out: string[] = [];
  for (const raw of lines.slice(1)) {
    const l = raw.replace(/\r$/, "");
    if (l === "push" || l.startsWith("push ")) st.push(parse(l.slice(5)));
    else if (l === "pop") out.push(show(st.pop()));
    else if (l === "min") out.push(show(st.min()));
    else if (l === "max") out.push(show(st.max()));
    else if (l === "size") out.push(String(st.size()));
  }
  if (out.length > 0) fs.writeSync(1, Buffer.from(out.join("\n") + "\n", "latin1"));
};
if (kind === "int") run(new MinMaxStack<bigint>((a, b) => (a < b ? -1 : a > b ? 1 : 0)), (s) => BigInt(s.trim()));
else run(new MinMaxStack<string>((a, b) => (a < b ? -1 : a > b ? 1 : 0)), (s) => s);
