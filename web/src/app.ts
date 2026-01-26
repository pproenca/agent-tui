import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";

const apiInput = document.getElementById("apiUrl") as HTMLInputElement;
const wsInput = document.getElementById("wsUrl") as HTMLInputElement;
const tokenInput = document.getElementById("token") as HTMLInputElement;
const sessionInput = document.getElementById("session") as HTMLInputElement;
const encodingSelect = document.getElementById("encoding") as HTMLSelectElement;
const connectBtn = document.getElementById("connectBtn") as HTMLButtonElement;
const statusEl = document.getElementById("status") as HTMLDivElement;
const statusText = statusEl.querySelector(".status__text") as HTMLSpanElement;
const metaEl = document.getElementById("meta") as HTMLDivElement;

const params = new URLSearchParams(window.location.search);
const decoder = new TextDecoder();

apiInput.value = params.get("api") ?? "";
wsInput.value = params.get("ws") ?? "";
tokenInput.value = params.get("token") ?? "";
sessionInput.value = params.get("session") ?? "active";
encodingSelect.value = params.get("encoding") ?? "binary";

const term = new Terminal({
  fontFamily: '"IBM Plex Mono", monospace',
  fontSize: 14,
  cursorBlink: false,
  scrollback: 2000,
  theme: {
    background: "#11100e",
    foreground: "#f7f1e6",
    cursor: "#f7f1e6",
  },
});
const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(document.getElementById("terminal") as HTMLDivElement);
fitAddon.fit();
window.addEventListener("resize", () => fitAddon.fit());

let socket: WebSocket | null = null;

function setStatus(text: string, connected: boolean) {
  statusText.textContent = text;
  if (connected) {
    statusEl.classList.add("status--connected");
  } else {
    statusEl.classList.remove("status--connected");
  }
}

function setMeta(text: string) {
  metaEl.textContent = text;
}

function resolveWsBase(): string | null {
  const wsValue = wsInput.value.trim();
  if (wsValue) {
    return wsValue;
  }
  const apiValue = apiInput.value.trim();
  if (!apiValue) {
    return null;
  }
  try {
    const api = new URL(apiValue);
    const wsProtocol = api.protocol === "https:" ? "wss:" : "ws:";
    return `${wsProtocol}//${api.host}/api/v1/stream`;
  } catch {
    return null;
  }
}

function buildWsUrl(): string | null {
  const base = resolveWsBase();
  if (!base) {
    return null;
  }
  const url = new URL(base);
  url.searchParams.set("session", sessionInput.value.trim() || "active");
  url.searchParams.set("encoding", encodingSelect.value || "binary");
  const token = tokenInput.value.trim();
  if (token) {
    url.searchParams.set("token", token);
  }
  return url.toString();
}

function decodeBase64(data: string): string {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return decoder.decode(bytes, { stream: true });
}

function handleTextEvent(text: string) {
  let payload: any;
  try {
    payload = JSON.parse(text);
  } catch {
    return;
  }

  switch (payload.event) {
    case "hello":
      setMeta(`API v${payload.api_version} · daemon ${payload.daemon_version}`);
      break;
    case "ready":
      if (payload.cols && payload.rows) {
        term.resize(payload.cols, payload.rows);
      }
      break;
    case "init":
      if (payload.init) {
        term.reset();
        term.write(payload.init);
      }
      break;
    case "output":
      if (payload.data_b64) {
        term.write(decodeBase64(payload.data_b64));
      } else if (payload.data) {
        term.write(decodeBase64(payload.data));
      }
      break;
    case "resize":
      if (payload.cols && payload.rows) {
        term.resize(payload.cols, payload.rows);
      }
      break;
    case "dropped":
      if (payload.dropped_bytes) {
        setMeta(`Dropped ${payload.dropped_bytes} bytes from stream.`);
      }
      break;
    case "error":
      if (payload.message) {
        setMeta(`Error: ${payload.message}`);
      }
      break;
    case "closed":
      setMeta("Stream closed.");
      break;
    default:
      break;
  }
}

function handleBinaryEvent(buffer: ArrayBuffer) {
  const bytes = new Uint8Array(buffer);
  if (bytes.length === 0) {
    return;
  }
  if (bytes[0] !== 0x01) {
    return;
  }
  const chunk = bytes.slice(1);
  term.write(decoder.decode(chunk, { stream: true }));
}

function disconnect() {
  if (socket) {
    socket.close();
    socket = null;
  }
  connectBtn.textContent = "Connect";
  setStatus("Disconnected", false);
}

function connect() {
  const wsUrl = buildWsUrl();
  if (!wsUrl) {
    setMeta("Provide an API or WS URL.");
    return;
  }

  disconnect();
  setStatus("Connecting...", false);
  setMeta(wsUrl);

  socket = new WebSocket(wsUrl);
  socket.binaryType = "arraybuffer";

  socket.addEventListener("open", () => {
    setStatus("Connected", true);
    connectBtn.textContent = "Disconnect";
  });

  socket.addEventListener("message", (event) => {
    if (typeof event.data === "string") {
      handleTextEvent(event.data);
      return;
    }
    if (event.data instanceof ArrayBuffer) {
      handleBinaryEvent(event.data);
    }
  });

  socket.addEventListener("close", () => {
    disconnect();
  });

  socket.addEventListener("error", () => {
    setMeta("Socket error.");
    disconnect();
  });
}

connectBtn.addEventListener("click", () => {
  if (socket) {
    disconnect();
  } else {
    connect();
  }
});

const auto = params.get("auto");
if (auto === "1" || auto === "true") {
  connect();
}
