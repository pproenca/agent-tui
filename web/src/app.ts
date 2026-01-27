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
const settingsDialog = document.getElementById("settingsDialog") as HTMLDialogElement | null;
const settingsBtn = document.getElementById("settingsBtn") as HTMLButtonElement | null;
const settingsClose = document.getElementById("settingsClose") as HTMLButtonElement | null;

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

function openSettings() {
  if (!settingsDialog) {
    return;
  }
  if (settingsDialog.open) {
    return;
  }
  settingsDialog.showModal();
}

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

async function hydrateLocalState(): Promise<boolean> {
  try {
    const resp = await fetch("/api-state");
    if (!resp.ok) {
      return false;
    }
    const payload = (await resp.json()) as {
      http_url?: string;
      ws_url?: string;
      token?: string;
    };
    let updated = false;
    if (!apiInput.value && payload.http_url) {
      apiInput.value = payload.http_url;
      updated = true;
    }
    if (!wsInput.value && payload.ws_url) {
      wsInput.value = payload.ws_url;
      updated = true;
    }
    if (!tokenInput.value && payload.token) {
      tokenInput.value = payload.token;
      updated = true;
    }
    return updated;
  } catch {
    return false;
  }
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

async function connect() {
  const hydrated = await hydrateLocalState();
  if (hydrated) {
    setMeta("Loaded local daemon settings.");
  }
  const wsUrl = buildWsUrl();
  if (!wsUrl) {
    setMeta("Provide an API or WS URL.");
    openSettings();
    return;
  }
  const tokenMissing = !tokenInput.value.trim();
  disconnect();
  setStatus("Connecting...", false);
  if (tokenMissing) {
    setMeta(`Token missing (ok if auth is disabled): ${wsUrl}`);
  } else {
    setMeta(wsUrl);
  }

  socket = new WebSocket(wsUrl);
  socket.binaryType = "arraybuffer";

  socket.addEventListener("open", () => {
    setStatus("Connected", true);
    connectBtn.textContent = "Disconnect";
    if (settingsDialog?.open) {
      settingsDialog.close();
    }
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
    void connect();
  }
});

settingsBtn?.addEventListener("click", () => {
  openSettings();
});

settingsClose?.addEventListener("click", () => {
  settingsDialog?.close();
});

settingsDialog?.addEventListener("click", (event) => {
  if (event.target === settingsDialog) {
    settingsDialog.close();
  }
});

async function init() {
  const hydrated = await hydrateLocalState();
  const auto = params.get("auto");
  if (auto === "1" || auto === "true") {
    void connect();
    return;
  }
  if (hydrated) {
    setMeta("Loaded local daemon settings.");
  }
}

void init();
