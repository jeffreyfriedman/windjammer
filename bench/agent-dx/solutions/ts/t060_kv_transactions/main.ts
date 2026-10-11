import * as fs from "node:fs";

const store = new Map<string, string>();
const txs: Map<string, string | undefined>[] = [];
const out: string[] = [];

const record = (k: string) => {
  const tx = txs[txs.length - 1];
  if (tx && !tx.has(k)) tx.set(k, store.get(k));
};

for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  switch (p[0]) {
    case "set":
      record(p[1]);
      store.set(p[1], String(BigInt(p[2])));
      break;
    case "get":
      out.push(store.get(p[1]) ?? "NULL");
      break;
    case "delete":
      record(p[1]);
      store.delete(p[1]);
      break;
    case "count": {
      const v = String(BigInt(p[1]));
      let n = 0;
      for (const x of store.values()) if (x === v) n++;
      out.push(String(n));
      break;
    }
    case "begin":
      txs.push(new Map());
      break;
    case "rollback": {
      const tx = txs.pop();
      if (!tx) out.push("NO TRANSACTION");
      else for (const [k, v] of tx) v === undefined ? store.delete(k) : store.set(k, v);
      break;
    }
    case "commit":
      if (txs.length === 0) out.push("NO TRANSACTION");
      else txs.length = 0;
      break;
  }
}
if (out.length > 0) console.log(out.join("\n"));
