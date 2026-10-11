import * as fs from "node:fs";

class Channel<T> {
  private buf: T[] = [];
  private closed = false;
  private recvWaiters: (() => void)[] = [];
  private sendWaiters: (() => void)[] = [];
  private cap: number;
  constructor(cap: number) {
    this.cap = cap;
  }
  async send(x: T): Promise<void> {
    while (this.buf.length >= this.cap) await new Promise<void>((r) => this.sendWaiters.push(r));
    this.buf.push(x);
    this.recvWaiters.shift()?.();
  }
  async recv(): Promise<T | undefined> {
    while (this.buf.length === 0) {
      if (this.closed) return undefined;
      await new Promise<void>((r) => this.recvWaiters.push(r));
    }
    const x = this.buf.shift()!;
    this.sendWaiters.shift()?.();
    return x;
  }
  close(): void {
    this.closed = true;
    for (const r of this.recvWaiters.splice(0)) r();
  }
}

const [P, K, C] = fs.readFileSync(0, "utf8").trim().split(/\s+/).map(Number);
const ch = new Channel<string>(4);
const received: string[] = [];
const counts: number[] = new Array(C).fill(0);
let chars = 0;

const producer = async (p: number) => {
  for (let j = 1; j <= K; j++) await ch.send(`p${p}-${j}`);
};
const consumer = async (c: number) => {
  for (;;) {
    const x = await ch.recv();
    if (x === undefined) return;
    counts[c]++;
    chars += x.length;
    received.push(x);
  }
};
const cons = Array.from({ length: C }, (_, c) => consumer(c));
await Promise.all(Array.from({ length: P }, (_, p) => producer(p)));
ch.close();
await Promise.all(cons);
received.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
const out = [`items: ${P * K}`, `chars: ${chars}`];
for (const x of received.slice(0, 3)) out.push(`first: ${x}`);
out.push(`consumers_total: ${counts.reduce((a, b) => a + b, 0)}`);
console.log(out.join("\n"));
