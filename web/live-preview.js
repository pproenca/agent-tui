#!/usr/bin/env node
"use strict";

const http = require("http");
const net = require("net");
const fs = require("fs");
const path = require("path");
const os = require("os");
const crypto = require("crypto");
const { TextDecoder } = require("util");

const MAGIC_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const DEFAULT_MAX_VIEWERS = 3;
const DEFAULT_MAX_QUEUE_BYTES = 2 * 1024 * 1024;

function parseArgs(argv) {
  const args = {
    listen: "127.0.0.1:0",
    allowRemote: false,
    stateFile: null,
    assetsDir: null,
    socketPath: null,
    maxViewers: null,
    maxQueueBytes: null,
  };

  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--listen" || arg === "-l") {
      args.listen = argv[i + 1] ?? args.listen;
      i += 1;
    } else if (arg === "--allow-remote") {
      args.allowRemote = true;
    } else if (arg === "--state-file") {
      args.stateFile = argv[i + 1] ?? null;
      i += 1;
    } else if (arg === "--assets") {
      args.assetsDir = argv[i + 1] ?? null;
      i += 1;
    } else if (arg === "--socket") {
      args.socketPath = argv[i + 1] ?? null;
      i += 1;
    } else if (arg === "--max-viewers") {
      args.maxViewers = argv[i + 1] ?? null;
      i += 1;
    } else if (arg === "--max-queue-bytes") {
      args.maxQueueBytes = argv[i + 1] ?? null;
      i += 1;
    }
  }

  return args;
}

function defaultSocketPath() {
  if (process.env.AGENT_TUI_SOCKET) {
    return process.env.AGENT_TUI_SOCKET;
  }
  if (process.env.XDG_RUNTIME_DIR) {
    return path.join(process.env.XDG_RUNTIME_DIR, "agent-tui.sock");
  }
  return "/tmp/agent-tui.sock";
}

function defaultStateFile() {
  const home = process.env.HOME || os.tmpdir();
  return path.join(home, ".agent-tui", "live-preview.json");
}

function parsePositiveInt(value, fallback) {
  if (value == null) {
    return fallback;
  }
  const parsed = Number.parseInt(value, 10);
  if (Number.isNaN(parsed) || parsed < 0) {
    return fallback;
  }
  return parsed;
}

function ensureDir(filePath) {
  const dir = path.dirname(filePath);
  fs.mkdirSync(dir, { recursive: true });
}

function parseListen(value) {
  if (!value) {
    throw new Error("listen address is empty");
  }

  if (value.startsWith("[")) {
    const end = value.indexOf("]");
    if (end === -1) {
      throw new Error(`invalid IPv6 listen address: ${value}`);
    }
    const host = value.slice(1, end);
    const portPart = value.slice(end + 1);
    if (!portPart.startsWith(":")) {
      throw new Error(`invalid listen address: ${value}`);
    }
    const port = Number.parseInt(portPart.slice(1), 10);
    if (Number.isNaN(port)) {
      throw new Error(`invalid port in listen address: ${value}`);
    }
    return { host, port };
  }

  const idx = value.lastIndexOf(":");
  if (idx <= 0) {
    throw new Error(`invalid listen address: ${value}`);
  }
  const host = value.slice(0, idx);
  const port = Number.parseInt(value.slice(idx + 1), 10);
  if (Number.isNaN(port)) {
    throw new Error(`invalid port in listen address: ${value}`);
  }
  return { host, port };
}

function isLoopback(host) {
  return host === "127.0.0.1" || host === "::1" || host === "localhost";
}

function loadAssets(assetsDir) {
  const html = fs.readFileSync(path.join(assetsDir, "index.html"), "utf8");
  const js = fs.readFileSync(
    path.join(assetsDir, "asciinema-player.min.js"),
    "utf8"
  );
  const css = fs.readFileSync(
    path.join(assetsDir, "asciinema-player.css"),
    "utf8"
  );
  return { html, js, css };
}

function createAcceptKey(key) {
  return crypto.createHash("sha1").update(key + MAGIC_GUID).digest("base64");
}

function buildWsFrame(opcode, payload) {
  const len = payload.length;
  let header;
  if (len < 126) {
    header = Buffer.alloc(2);
    header[1] = len;
  } else if (len < 65536) {
    header = Buffer.alloc(4);
    header[1] = 126;
    header.writeUInt16BE(len, 2);
  } else {
    header = Buffer.alloc(10);
    header[1] = 127;
    header.writeBigUInt64BE(BigInt(len), 2);
  }
  header[0] = 0x80 | (opcode & 0x0f);
  return Buffer.concat([header, payload]);
}

function createWsSender(socket, maxQueueBytes) {
  let queue = [];
  let queuedBytes = 0;
  let draining = false;
  let closed = false;

  function enqueue(frame) {
    if (closed) {
      return false;
    }
    if (draining) {
      if (queuedBytes + frame.length > maxQueueBytes) {
        return false;
      }
      queue.push(frame);
      queuedBytes += frame.length;
      return true;
    }
    const ok = socket.write(frame);
    if (!ok) {
      draining = true;
      socket.once("drain", flush);
    }
    return true;
  }

  function flush() {
    if (closed) {
      return;
    }
    draining = false;
    while (queue.length > 0) {
      const frame = queue.shift();
      queuedBytes -= frame.length;
      const ok = socket.write(frame);
      if (!ok) {
        draining = true;
        socket.once("drain", flush);
        break;
      }
    }
  }

  function sendText(text) {
    const payload = Buffer.from(text, "utf8");
    return enqueue(buildWsFrame(0x1, payload));
  }

  function sendPong(payload) {
    return enqueue(buildWsFrame(0xa, payload));
  }

  function sendClose() {
    if (closed) {
      return;
    }
    closed = true;
    const frame = buildWsFrame(0x8, Buffer.alloc(0));
    socket.write(frame);
    socket.end();
  }

  function closeNow() {
    if (closed) {
      return;
    }
    closed = true;
    socket.destroy();
  }

  return {
    sendText,
    sendPong,
    sendClose,
    closeNow,
    isClosed: () => closed,
  };
}

function createRpcClient(socketPath) {
  return function callRpc(method, params) {
    return new Promise((resolve, reject) => {
      const socket = net.createConnection(socketPath);
      let buffer = "";
      let settled = false;

      socket.on("connect", () => {
        const request = {
          jsonrpc: "2.0",
          id: 1,
          method,
          params: params ?? null,
        };
        socket.write(JSON.stringify(request) + "\n");
      });

      socket.on("data", (chunk) => {
        buffer += chunk.toString("utf8");
        let idx = buffer.indexOf("\n");
        while (idx !== -1) {
          const line = buffer.slice(0, idx).trim();
          buffer = buffer.slice(idx + 1);
          idx = buffer.indexOf("\n");
          if (!line) {
            continue;
          }
          try {
            const payload = JSON.parse(line);
            if (payload.error) {
              settled = true;
              socket.end();
              reject(
                new Error(payload.error.message || "daemon RPC error")
              );
              return;
            }
            settled = true;
            socket.end();
            resolve(payload.result);
            return;
          } catch (err) {
            settled = true;
            socket.end();
            reject(err);
            return;
          }
        }
      });

      socket.on("error", (err) => {
        if (!settled) {
          settled = true;
          reject(err);
        }
      });

      socket.on("end", () => {
        if (!settled) {
          settled = true;
          reject(new Error("daemon connection closed"));
        }
      });
    });
  };
}

function startPreviewStream(socketPath, session, onEvent, onError, onEnd) {
  const socket = net.createConnection(socketPath);
  let buffer = "";

  socket.on("connect", () => {
    const request = {
      jsonrpc: "2.0",
      id: 1,
      method: "live_preview_stream",
      params: session ? { session } : null,
    };
    socket.write(JSON.stringify(request) + "\n");
  });

  socket.on("data", (chunk) => {
    buffer += chunk.toString("utf8");
    let idx = buffer.indexOf("\n");
    while (idx !== -1) {
      const line = buffer.slice(0, idx).trim();
      buffer = buffer.slice(idx + 1);
      idx = buffer.indexOf("\n");
      if (!line) {
        continue;
      }
      try {
        const payload = JSON.parse(line);
        if (payload.error) {
          onError(new Error(payload.error.message || "daemon error"));
          socket.end();
          return;
        }
        if (payload.result) {
          onEvent(payload.result);
        }
      } catch (err) {
        onError(err);
        socket.end();
        return;
      }
    }
  });

  socket.on("error", onError);
  socket.on("end", onEnd);

  return () => {
    socket.end();
  };
}

function handleWebSocket(socket, request, socketPath, maxQueueBytes) {
  const url = new URL(request.url, "http://localhost");
  const session = url.searchParams.get("session") || null;
  let decoder = new TextDecoder("utf-8");
  let streamClosed = false;
  let reconnectAttempts = 0;
  let reconnectTimer = null;

  const sender = createWsSender(socket, maxQueueBytes);

  function stopReconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  }

  function scheduleReconnect() {
    if (streamClosed || sender.isClosed()) {
      return;
    }
    if (reconnectTimer) {
      return;
    }
    const delay = Math.min(500 * 2 ** reconnectAttempts, 5000);
    reconnectAttempts += 1;
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      if (!streamClosed && !sender.isClosed()) {
        connectStream();
      }
    }, delay);
  }

  function closeDueToBackpressure() {
    if (streamClosed) {
      return;
    }
    streamClosed = true;
    if (stopStream) {
      stopStream();
    }
    sender.sendClose();
  }

  let stopStream = null;
  function connectStream() {
    if (streamClosed) {
      return;
    }
    stopStream = startPreviewStream(
      socketPath,
      session,
      (event) => {
        if (streamClosed) {
          return;
        }
        if (!event || !event.event) {
          return;
        }
        if (event.event === "ready") {
          reconnectAttempts = 0;
          return;
        }
        switch (event.event) {
          case "init": {
            reconnectAttempts = 0;
            decoder = new TextDecoder("utf-8");
            const ok = sender.sendText(
              JSON.stringify({
                time: event.time ?? 0,
                cols: event.cols,
                rows: event.rows,
                init: event.init,
              })
            );
            if (!ok) {
              closeDueToBackpressure();
            }
            break;
          }
          case "output": {
            if (!event.data_b64) {
              break;
            }
            const buf = Buffer.from(event.data_b64, "base64");
            const text = decoder.decode(buf, { stream: true });
            if (text.length > 0) {
              const ok = sender.sendText(
                JSON.stringify([event.time ?? 0, "o", text])
              );
              if (!ok) {
                closeDueToBackpressure();
              }
            }
            break;
          }
          case "resize": {
            if (event.cols && event.rows) {
              const ok = sender.sendText(
                JSON.stringify([event.time ?? 0, "r", `${event.cols}x${event.rows}`])
              );
              if (!ok) {
                closeDueToBackpressure();
              }
            }
            break;
          }
          case "closed": {
            streamClosed = true;
            sender.sendClose();
            break;
          }
          default:
            break;
        }
      },
      () => {
        if (streamClosed) {
          return;
        }
        scheduleReconnect();
      },
      () => {
        if (streamClosed) {
          return;
        }
        scheduleReconnect();
      }
    );
  }

  connectStream();

  let frameBuffer = Buffer.alloc(0);

  socket.on("data", (chunk) => {
    frameBuffer = Buffer.concat([frameBuffer, chunk]);
    while (frameBuffer.length >= 2) {
      const first = frameBuffer[0];
      const second = frameBuffer[1];
      const fin = (first & 0x80) !== 0;
      const opcode = first & 0x0f;
      const masked = (second & 0x80) !== 0;
      let payloadLen = second & 0x7f;
      let offset = 2;

      if (payloadLen === 126) {
        if (frameBuffer.length < offset + 2) {
          break;
        }
        payloadLen = frameBuffer.readUInt16BE(offset);
        offset += 2;
      } else if (payloadLen === 127) {
        if (frameBuffer.length < offset + 8) {
          break;
        }
        const hi = frameBuffer.readUInt32BE(offset);
        const lo = frameBuffer.readUInt32BE(offset + 4);
        payloadLen = hi * 2 ** 32 + lo;
        offset += 8;
      }

      if (masked) {
        if (frameBuffer.length < offset + 4) {
          break;
        }
      }

      if (frameBuffer.length < offset + (masked ? 4 : 0) + payloadLen) {
        break;
      }

      let mask = null;
      if (masked) {
        mask = frameBuffer.slice(offset, offset + 4);
        offset += 4;
      }
      let payload = frameBuffer.slice(offset, offset + payloadLen);
      offset += payloadLen;
      frameBuffer = frameBuffer.slice(offset);

      if (masked && mask) {
        const unmasked = Buffer.alloc(payload.length);
        for (let i = 0; i < payload.length; i += 1) {
          unmasked[i] = payload[i] ^ mask[i % 4];
        }
        payload = unmasked;
      }

      if (!fin) {
        continue;
      }

      if (opcode === 0x8) {
        streamClosed = true;
        stopReconnect();
        if (stopStream) {
          stopStream();
        }
        sender.sendClose();
        return;
      }
      if (opcode === 0x9) {
        const ok = sender.sendPong(payload);
        if (!ok) {
          closeDueToBackpressure();
          return;
        }
      }
    }
  });

  socket.on("close", () => {
    streamClosed = true;
    stopReconnect();
    if (stopStream) {
      stopStream();
    }
  });

  socket.on("error", () => {
    streamClosed = true;
    stopReconnect();
    if (stopStream) {
      stopStream();
    }
  });
}

function main() {
  const args = parseArgs(process.argv);
  const socketPath = args.socketPath || defaultSocketPath();
  const stateFile = args.stateFile || defaultStateFile();
  const assetsDir =
    args.assetsDir ||
    path.resolve(__dirname, "assets", "live");
  const maxViewers = parsePositiveInt(
    args.maxViewers ?? process.env.AGENT_TUI_LIVE_MAX_VIEWERS,
    DEFAULT_MAX_VIEWERS
  );
  const maxQueueBytes = parsePositiveInt(
    args.maxQueueBytes ?? process.env.AGENT_TUI_LIVE_MAX_QUEUE_BYTES,
    DEFAULT_MAX_QUEUE_BYTES
  );

  const listen = parseListen(args.listen);
  if (!args.allowRemote && !isLoopback(listen.host)) {
    console.error(
      `Refusing to bind ${listen.host}. Pass --allow-remote to enable.`
    );
    process.exit(2);
  }

  let assets;
  try {
    assets = loadAssets(assetsDir);
  } catch (err) {
    console.error(`Failed to load assets from ${assetsDir}: ${err.message}`);
    process.exit(2);
  }

  const callRpc = createRpcClient(socketPath);

  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, "http://localhost");
    if (url.pathname === "/" || url.pathname === "/index.html") {
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(assets.html);
      return;
    }
    if (url.pathname === "/asciinema-player.min.js") {
      res.writeHead(200, { "Content-Type": "application/javascript" });
      res.end(assets.js);
      return;
    }
    if (url.pathname === "/asciinema-player.css") {
      res.writeHead(200, { "Content-Type": "text/css" });
      res.end(assets.css);
      return;
    }
    if (url.pathname === "/sessions") {
      try {
        const result = await callRpc("sessions", null);
        const sessions = (result.sessions || []).map((session) => ({
          id: session.id,
          command: session.command,
          pid: session.pid,
          running: session.running,
          created_at: session.created_at,
          cols: session.size ? session.size.cols : session.cols,
          rows: session.size ? session.size.rows : session.rows,
        }));
        const payload = {
          active: result.active_session || result.active || null,
          sessions,
        };
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify(payload));
      } catch (err) {
        res.writeHead(502, { "Content-Type": "application/json" });
        res.end(
          JSON.stringify({ error: "Failed to fetch sessions", details: err.message })
        );
      }
      return;
    }
    res.writeHead(404, { "Content-Type": "text/plain" });
    res.end("Not found");
  });

  let activeViewers = 0;

  server.on("upgrade", (req, socket) => {
    const url = new URL(req.url, "http://localhost");
    if (url.pathname !== "/ws/alis") {
      socket.destroy();
      return;
    }
    if (maxViewers > 0 && activeViewers >= maxViewers) {
      socket.write(
        "HTTP/1.1 503 Service Unavailable\r\n" +
          "Connection: close\r\n" +
          "Content-Type: text/plain\r\n" +
          "Retry-After: 1\r\n" +
          "\r\n" +
          "Live preview is at capacity. Try again later.\n"
      );
      socket.destroy();
      return;
    }
    const key = req.headers["sec-websocket-key"];
    if (!key) {
      socket.destroy();
      return;
    }
    const accept = createAcceptKey(key);
    const headers = [
      "HTTP/1.1 101 Switching Protocols",
      "Upgrade: websocket",
      "Connection: Upgrade",
      `Sec-WebSocket-Accept: ${accept}`,
      "\r\n",
    ];
    socket.write(headers.join("\r\n"));
    activeViewers += 1;
    let counted = true;
    const onClose = () => {
      if (counted) {
        counted = false;
        activeViewers = Math.max(0, activeViewers - 1);
      }
    };
    socket.on("close", onClose);
    socket.on("error", onClose);
    handleWebSocket(socket, req, socketPath, maxQueueBytes);
  });

  server.listen(listen.port, listen.host, () => {
    const address = server.address();
    const host =
      typeof address === "string" ? address : address.address || listen.host;
    const port =
      typeof address === "string" ? listen.port : address.port || listen.port;
    const displayHost = host.includes(":") ? `[${host}]` : host;
    const url = `http://${displayHost}:${port}/`;

    try {
      ensureDir(stateFile);
      fs.writeFileSync(
        stateFile,
        JSON.stringify(
          {
            pid: process.pid,
            url,
            listen: `${host}:${port}`,
            socket: socketPath,
            assets: assetsDir,
            started_at: new Date().toISOString(),
          },
          null,
          2
        )
      );
    } catch (err) {
      console.error(`Failed to write state file: ${err.message}`);
    }

    console.log(url);
  });

  function cleanup() {
    try {
      if (fs.existsSync(stateFile)) {
        fs.unlinkSync(stateFile);
      }
    } catch (_) {}
    process.exit(0);
  }

  process.on("SIGINT", cleanup);
  process.on("SIGTERM", cleanup);
}

main();
