// Windjammer std::subprocess backend for Node.js (child_process).
import { spawn } from "node:child_process";
import readline from "node:readline";

const sessions = new Map();
let nextId = 1;

export function spawnProcess(program, args = []) {
  const child = spawn(program, args, { stdio: ["pipe", "pipe", "inherit"] });
  const id = nextId++;
  const rl = readline.createInterface({ input: child.stdout });
  sessions.set(id, { child, rl });
  return { id };
}

export function writeLine(handle, line) {
  const s = sessions.get(handle.id);
  if (!s) throw new Error("invalid handle");
  s.child.stdin.write(line + "\n");
}

export async function readLine(handle) {
  const s = sessions.get(handle.id);
  if (!s) throw new Error("invalid handle");
  for await (const line of s.rl) return line;
  throw new Error("stdout closed");
}
