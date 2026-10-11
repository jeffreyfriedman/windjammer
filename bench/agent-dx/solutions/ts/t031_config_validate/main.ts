import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const errs: string[] = [];
const vals = new Map<string, string>();
const order: string[] = [];
lines.forEach((raw, i) => {
  const l = raw.replace(/\r$/, "");
  if (l === "") return;
  const e = l.indexOf("=");
  if (e < 0) {
    errs.push(`error: malformed line ${i + 1}`);
    return;
  }
  const k = l.slice(0, e);
  if (!vals.has(k)) order.push(k);
  vals.set(k, l.slice(e + 1));
});
const rangeInt = (v: string, lo: number, hi: number): number | null => {
  if (!/^[0-9]+$/.test(v)) return null;
  const n = BigInt(v);
  return n >= BigInt(lo) && n <= BigInt(hi) ? Number(n) : null;
};
const host = vals.get("host");
if (host === undefined) errs.push("error: host: missing");
else if (host === "") errs.push("error: host: empty");
let port = 0;
const pv = vals.get("port");
if (pv === undefined) errs.push("error: port: missing");
else {
  const n = rangeInt(pv, 1, 65535);
  if (n === null) errs.push(`error: port: invalid value ${pv}`);
  else port = n;
}
let debug = "false";
const dv = vals.get("debug");
if (dv !== undefined) {
  if (dv === "true" || dv === "false") debug = dv;
  else errs.push(`error: debug: invalid value ${dv}`);
}
let workers = 4;
const wv = vals.get("workers");
if (wv !== undefined) {
  const n = rangeInt(wv, 1, 64);
  if (n === null) errs.push(`error: workers: invalid value ${wv}`);
  else workers = n;
}
for (const k of order) {
  if (!["host", "port", "debug", "workers"].includes(k)) errs.push(`error: unknown key ${k}`);
}
if (errs.length) console.log(errs.join("\n"));
else console.log(`ok host=${host} port=${port} debug=${debug} workers=${workers}`);
