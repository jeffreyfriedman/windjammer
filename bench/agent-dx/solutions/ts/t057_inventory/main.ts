import * as fs from "node:fs";

const inv = new Map<string, { qty: bigint; price: bigint }>();
const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "") continue;
  if (p[0] === "stock") {
    const qty = BigInt(p[2]), price = BigInt(p[3]);
    if (qty <= 0n) out.push("error: bad quantity");
    else {
      const it = inv.get(p[1]);
      if (!it) {
        inv.set(p[1], { qty, price });
        out.push(`ok ${qty}`);
      } else if (it.price !== price) out.push(`error: price mismatch for ${p[1]}`);
      else {
        it.qty += qty;
        out.push(`ok ${it.qty}`);
      }
    }
  } else if (p[0] === "sell") {
    const qty = BigInt(p[2]);
    const it = inv.get(p[1]);
    if (qty <= 0n) out.push("error: bad quantity");
    else if (!it) out.push(`error: unknown sku ${p[1]}`);
    else if (it.qty < qty) out.push(`error: insufficient stock for ${p[1]} (have ${it.qty}, need ${qty})`);
    else {
      it.qty -= qty;
      out.push(`sold ${qty} for ${qty * it.price}`);
    }
  } else if (p[0] === "remove") {
    const it = inv.get(p[1]);
    if (!it) out.push(`error: unknown sku ${p[1]}`);
    else if (it.qty !== 0n) out.push(`error: not empty ${p[1]}`);
    else {
      inv.delete(p[1]);
      out.push(`removed ${p[1]}`);
    }
  } else if (p[0] === "report") {
    if (inv.size === 0) out.push("(empty)");
    else {
      let total = 0n;
      for (const k of [...inv.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))) {
        const it = inv.get(k)!;
        total += it.qty * it.price;
        out.push(`${k} ${it.qty} x ${it.price} = ${it.qty * it.price}`);
      }
      out.push(`total value ${total}`);
    }
  }
}
if (out.length > 0) console.log(out.join("\n"));
