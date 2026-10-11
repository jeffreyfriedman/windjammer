import * as fs from "node:fs";

const data = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0).map(Number);
const N = 2000000;
const comp = new Uint8Array(N + 1);
const cnt = new Int32Array(N + 1);
const last = new Int32Array(N + 1);
for (let i = 2; i <= N; i++) {
  if (!comp[i]) for (let j = i * i; j <= N; j += i) comp[j] = 1;
}
for (let i = 2; i <= N; i++) {
  const p = !comp[i];
  cnt[i] = cnt[i - 1] + (p ? 1 : 0);
  last[i] = p ? i : last[i - 1];
}
const out: string[] = [];
for (let i = 0; i < data[0]; i++) {
  const n = data[1 + i];
  out.push(n < 2 ? "0 none" : `${cnt[n]} ${last[n]}`);
}
if (out.length > 0) console.log(out.join("\n"));
