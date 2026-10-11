import * as fs from "node:fs";

type E = { p: bigint; label: string; seq: number };
const h: E[] = [];
const less = (a: E, b: E) => a.p < b.p || (a.p === b.p && a.seq < b.seq);

function push(e: E) {
  h.push(e);
  let i = h.length - 1;
  while (i > 0) {
    const par = (i - 1) >> 1;
    if (!less(h[i], h[par])) break;
    [h[i], h[par]] = [h[par], h[i]];
    i = par;
  }
}

function pop(): E {
  const top = h[0];
  const last = h.pop()!;
  if (h.length > 0) {
    h[0] = last;
    let i = 0;
    for (;;) {
      const l = 2 * i + 1, r = l + 1;
      let s = i;
      if (l < h.length && less(h[l], h[s])) s = l;
      if (r < h.length && less(h[r], h[s])) s = r;
      if (s === i) break;
      [h[i], h[s]] = [h[s], h[i]];
      i = s;
    }
  }
  return top;
}

const out: string[] = [];
let seq = 0;
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "push") push({ p: BigInt(p[1]), label: p[2], seq: seq++ });
  else if (p[0] === "pop") {
    if (h.length === 0) out.push("empty");
    else {
      const e = pop();
      out.push(`${e.p} ${e.label}`);
    }
  } else if (p[0] === "peek") out.push(h.length ? `${h[0].p} ${h[0].label}` : "empty");
  else if (p[0] === "size") out.push(String(h.length));
}
if (out.length > 0) console.log(out.join("\n"));
