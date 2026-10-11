import * as fs from "node:fs";

const price: Record<string, number> = { cola: 150, chips: 100, gum: 35 };
const stock: Record<string, number> = { cola: 3, chips: 2, gum: 5 };
let credit = 0;

const change = (amt: number): string => {
  const coins: number[] = [];
  for (const c of [100, 25, 10, 5]) {
    while (amt >= c) {
      coins.push(c);
      amt -= c;
    }
  }
  return coins.length ? coins.join(" ") : "none";
};

const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "") continue;
  if (p[0] === "coin") {
    const v = Number(p[1]);
    if ([5, 10, 25, 100].includes(v)) {
      credit += v;
      out.push(`credit ${credit}`);
    } else out.push(`rejected ${p[1]}`);
  } else if (p[0] === "select") {
    const item = p[1];
    if (!Object.hasOwn(price, item)) out.push(`error: unknown item ${item}`);
    else if (stock[item] <= 0) out.push(`error: sold out ${item}`);
    else if (credit < price[item]) out.push(`error: need ${price[item] - credit} more`);
    else {
      stock[item]--;
      out.push(`dispensed ${item} change: ${change(credit - price[item])}`);
      credit = 0;
    }
  } else if (p[0] === "refund") {
    out.push(`refund: ${change(credit)}`);
    credit = 0;
  } else if (p[0] === "status") {
    out.push(`credit=${credit} cola=${stock.cola} chips=${stock.chips} gum=${stock.gum}`);
  }
}
if (out.length > 0) console.log(out.join("\n"));
