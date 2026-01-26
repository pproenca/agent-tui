#!/usr/bin/env bun
import { readFile } from "node:fs/promises";
import { homedir } from "node:os";

const args = new Map<string, string>();
for (const arg of process.argv.slice(2)) {
  const [key, value] = arg.split("=");
  if (key && value) {
    args.set(key.replace(/^--/, ""), value);
  }
}

const statePath =
  Bun.env.AGENT_TUI_API_STATE ?? `${homedir()}/.agent-tui/api.json`;
const stateRaw = await readFile(statePath, "utf8");
const state = JSON.parse(stateRaw);

const wsBase = args.get("ws") ?? state.ws_url;
const token = args.get("token") ?? state.token ?? "";
const session = args.get("session") ?? "active";
const encoding = args.get("encoding") ?? "binary";

const url = new URL(wsBase);
url.searchParams.set("session", session);
url.searchParams.set("encoding", encoding);
if (token) {
  url.searchParams.set("token", token);
}

const decoder = new TextDecoder();
const ws = new WebSocket(url.toString());
ws.binaryType = "arraybuffer";

ws.addEventListener("open", () => {
  console.log(`connected ${url}`);
});

ws.addEventListener("message", (event) => {
  if (typeof event.data === "string") {
    const payload = JSON.parse(event.data);
    if (payload.event === "init" && payload.init) {
      process.stdout.write(payload.init);
    }
    if (payload.event === "output" && payload.data_b64) {
      const decoded = Buffer.from(payload.data_b64, "base64");
      process.stdout.write(decoded);
    }
    return;
  }

  if (event.data instanceof ArrayBuffer) {
    const bytes = new Uint8Array(event.data);
    if (bytes[0] === 0x01) {
      const chunk = bytes.slice(1);
      process.stdout.write(decoder.decode(chunk, { stream: true }));
    }
  }
});

ws.addEventListener("close", () => {
  console.log("stream closed");
});
