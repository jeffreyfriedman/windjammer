// A static server shaped like GitHub Pages for the playground's browser tests (ADR-0034): serves docs/book/playground
// with the same media types and `Access-Control-Allow-Origin: *`, and records every request, so a test can prove a
// program reached nothing. Any path under /exfil is a trap: it should never be requested.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".txt": "text/plain; charset=utf-8",
};

/** Start a server for `root` on 127.0.0.1 (random port); returns {url, port, requests, close}. */
export async function serve(root) {
  const requests = [];
  const server = createServer(async (req, res) => {
    requests.push({ method: req.method, url: req.url });
    const path = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (path.startsWith("/exfil")) {
      res.writeHead(200, { "Access-Control-Allow-Origin": "*", "Content-Type": "text/javascript" });
      res.end("/* trap */");
      return;
    }
    const file = normalize(join(root, path.endsWith("/") ? path + "index.html" : path));
    if (!file.startsWith(normalize(root))) {
      res.writeHead(403).end();
      return;
    }
    try {
      const body = await readFile(file);
      res.writeHead(200, { "Content-Type": TYPES[extname(file)] || "application/octet-stream", "Access-Control-Allow-Origin": "*" });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  server.on("upgrade", (req, socket) => {
    requests.push({ method: "UPGRADE", url: req.url });
    socket.destroy();
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  const port = server.address().port;
  return {
    url: `http://127.0.0.1:${port}/`,
    port,
    requests,
    close: () => new Promise((ok) => server.close(ok)),
  };
}
