import { join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL(".", import.meta.url));
const publicDir = join(root, "public");
const port = Number.parseInt(Bun.env.PORT ?? "4173", 10);
const apiStatePath =
  Bun.env.AGENT_TUI_API_STATE ??
  join(Bun.env.HOME ?? "/tmp", ".agent-tui", "api.json");

function resolvePath(pathname: string): string {
  const cleaned = pathname === "/" ? "/index.html" : pathname;
  const normalized = normalize(cleaned).replace(/^\.\.(\/|\\|$)+/, "");
  return join(publicDir, normalized);
}

const server = Bun.serve({
  port,
  async fetch(req) {
    const url = new URL(req.url);
    if (url.pathname === "/api-state") {
      const file = Bun.file(apiStatePath);
      if (await file.exists()) {
        return new Response(file, {
          headers: {
            "content-type": "application/json",
            "cache-control": "no-store",
          },
        });
      }
      return new Response("Not found", { status: 404 });
    }
    const path = resolvePath(url.pathname);
    const file = Bun.file(path);
    if (await file.exists()) {
      return new Response(file);
    }
    return new Response("Not found", { status: 404 });
  },
});

console.log(`agent-tui web UI running on http://127.0.0.1:${server.port}`);
