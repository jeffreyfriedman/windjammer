import * as fs from "node:fs";

type Node = { k: bigint; l: Node | null; r: Node | null };
let root: Node | null = null;

function contains(k: bigint): boolean {
  let n = root;
  while (n) {
    if (k === n.k) return true;
    n = k < n.k ? n.l : n.r;
  }
  return false;
}

function insert(n: Node | null, k: bigint): Node {
  if (!n) return { k, l: null, r: null };
  if (k < n.k) n.l = insert(n.l, k);
  else n.r = insert(n.r, k);
  return n;
}

function del(n: Node | null, k: bigint): Node | null {
  if (!n) return null;
  if (k < n.k) n.l = del(n.l, k);
  else if (k > n.k) n.r = del(n.r, k);
  else {
    if (!n.l) return n.r;
    if (!n.r) return n.l;
    let s = n.r;
    while (s.l) s = s.l;
    n.k = s.k;
    n.r = del(n.r, s.k);
  }
  return n;
}

function inorder(n: Node | null, acc: string[]) {
  if (!n) return;
  inorder(n.l, acc);
  acc.push(String(n.k));
  inorder(n.r, acc);
}

function height(n: Node | null): number {
  return n ? 1 + Math.max(height(n.l), height(n.r)) : 0;
}

const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  switch (p[0]) {
    case "insert":
      if (contains(BigInt(p[1]))) out.push("dup");
      else {
        root = insert(root, BigInt(p[1]));
        out.push("ok");
      }
      break;
    case "contains":
      out.push(contains(BigInt(p[1])) ? "yes" : "no");
      break;
    case "delete":
      if (contains(BigInt(p[1]))) {
        root = del(root, BigInt(p[1]));
        out.push("ok");
      } else out.push("missing");
      break;
    case "inorder": {
      const acc: string[] = [];
      inorder(root, acc);
      out.push(acc.length ? acc.join(" ") : "<empty>");
      break;
    }
    case "height":
      out.push(String(height(root)));
      break;
    case "min":
    case "max": {
      let n = root;
      if (!n) out.push("empty");
      else {
        while (p[0] === "min" ? n.l : n.r) n = (p[0] === "min" ? n.l : n.r)!;
        out.push(String(n.k));
      }
      break;
    }
  }
}
if (out.length > 0) console.log(out.join("\n"));
