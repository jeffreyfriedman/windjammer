// The TypeScript sandbox (ADR-0034), inside <iframe sandbox="allow-scripts">: strips the types, runs the program in
// a Worker made from a blob: URL, and reports {stdout, stderr, exit_code} to the page. Only the embedding page may
// start a run; the Worker's messages are checked field by field and capped.
import { programSource, stripTypes, TS_DEFAULTS } from "./ts-core.mjs";

let current = null; // {id, worker, timer, stdout, stderr, bytes, outputBytes}

function reply(msg) {
  // the parent's origin is not visible from an opaque origin; the message carries only the program's own output
  window.parent.postMessage({ type: "wj-ts-result", ...msg }, "*");
}

function end(code, note) {
  if (!current) return;
  const run = current;
  current = null;
  clearTimeout(run.timer);
  run.worker.terminate();
  if (note) run.stderr += note;
  reply({ id: run.id, stdout: run.stdout, stderr: run.stderr, exit_code: code });
}

function start(id, source, wallMs, outputBytes) {
  if (current) end(124, "stopped: a new run started\n");
  let js;
  try {
    js = stripTypes(source);
  } catch (e) {
    reply({ id, stdout: "", stderr: `TypeScript: ${e && e.message ? e.message : e}\n`, exit_code: 1 });
    return;
  }
  const url = URL.createObjectURL(new Blob([programSource(js, { outputBytes })], { type: "text/javascript" }));
  const worker = new Worker(url);
  URL.revokeObjectURL(url);
  current = { id, worker, stdout: "", stderr: "", bytes: 0, outputBytes, timer: 0 };
  current.timer = setTimeout(() => end(124, `stopped after ${wallMs / 1000} s\n`), wallMs);
  worker.onmessage = (ev) => {
    const m = ev.data;
    if (!current || current.worker !== worker || !m || typeof m !== "object") return;
    if (m.k === "out" && typeof m.text === "string" && (m.stream === "stdout" || m.stream === "stderr")) {
      current.bytes += m.text.length;
      if (current.bytes > outputBytes) return end(124, "stopped: output limit\n");
      current[m.stream] += m.text;
    } else if (m.k === "limit") {
      end(124, "stopped: output limit\n");
    } else if (m.k === "done" && Number.isInteger(m.code)) {
      end(m.code);
    }
  };
  worker.onerror = (e) => {
    e.preventDefault();
    if (current && current.worker === worker) end(1, `Uncaught ${e.message || "error"}\n`);
  };
}

window.addEventListener("message", (ev) => {
  if (ev.source !== window.parent) return;
  const m = ev.data;
  if (!m || m.type !== "wj-ts-run" || typeof m.source !== "string" || !Number.isInteger(m.id)) return;
  const wallMs = Number.isFinite(m.wallMs) ? Math.min(Math.max(m.wallMs, 100), 60_000) : TS_DEFAULTS.wallMs;
  const outputBytes = Number.isFinite(m.outputBytes) ? Math.min(m.outputBytes, 8 << 20) : TS_DEFAULTS.outputBytes;
  start(m.id, m.source, wallMs, outputBytes);
});

window.parent.postMessage({ type: "wj-ts-ready" }, "*");
