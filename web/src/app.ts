import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";

const connectBtn = document.getElementById("connectBtn") as HTMLButtonElement;
const statusEl = document.getElementById("status") as HTMLDivElement;
const statusText = statusEl.querySelector(".status__text") as HTMLSpanElement;
const sessionListEl = document.getElementById("sessionList") as HTMLDivElement;
const sessionEmptyEl = document.getElementById("sessionEmpty") as HTMLDivElement;
const sessionsRefreshBtn = document.getElementById("sessionsRefresh") as
  | HTMLButtonElement
  | null;

const params = new URLSearchParams(window.location.search);
const decoder = new TextDecoder();

type Encoding = "binary" | "base64";
type ConnectionConfig = {
  apiUrl: string;
  wsUrl: string;
  token: string;
  session: string;
  encoding: Encoding;
};

const config: ConnectionConfig = {
  apiUrl: params.get("api") ?? "",
  wsUrl: params.get("ws") ?? "",
  token: params.get("token") ?? "",
  session: params.get("session") ?? "active",
  encoding: params.get("encoding") === "base64" ? "base64" : "binary",
};

if (!config.session.trim()) {
  config.session = "active";
}

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
let socketState: { reason: DisconnectReason | null } | null = null;
let closedNoticeSent = false;
let latestSessions: SessionsResponse | null = null;
let sessionsLoading = false;
let refreshInterval: ReturnType<typeof setInterval> | null = null;
const REFRESH_INTERVAL_MS = 5000;

type DisconnectReason = "manual" | "reconnect" | "server" | "error";
type SessionsResponse = {
  active?: string | null;
  sessions: SessionInfo[];
};

type SessionInfo = {
  id: string;
  command: string;
  pid: number;
  running: boolean;
  created_at: string;
  size: { cols: number; rows: number };
};

function setStatus(text: string, connected: boolean) {
  statusText.textContent = text;
  if (connected) {
    statusEl.classList.add("status--connected");
  } else {
    statusEl.classList.remove("status--connected");
  }
}

function normalizedSessionValue(value: string): string {
  const trimmed = value.trim();
  return trimmed.length === 0 ? "active" : trimmed;
}

function showTerminationNotice() {
  if (!closedNoticeSent) {
    closedNoticeSent = true;
    term.write("\r\n\x1b[90m[session terminated]\x1b[0m\r\n");
  }
}

function startPeriodicRefresh() {
  if (refreshInterval) return;
  refreshInterval = setInterval(() => {
    void refreshSessions();
  }, REFRESH_INTERVAL_MS);
}

function stopPeriodicRefresh() {
  if (refreshInterval) {
    clearInterval(refreshInterval);
    refreshInterval = null;
  }
}

function finalizeDisconnect(reason: DisconnectReason) {
  stopPeriodicRefresh();
  connectBtn.textContent = "+";
  connectBtn.title = "Connect";
  setStatus("Disconnected", false);
  if (reason === "server") {
    showTerminationNotice();
  }
}

function disconnect(reason: DisconnectReason = "manual") {
  if (!socket) {
    finalizeDisconnect(reason);
    return;
  }
  if (socketState) {
    socketState.reason = reason;
  }
  socket.close();
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
    if (!config.apiUrl && payload.http_url) {
      config.apiUrl = payload.http_url;
      updated = true;
    }
    if (!config.wsUrl && payload.ws_url) {
      config.wsUrl = payload.ws_url;
      updated = true;
    }
    if (!config.token && payload.token) {
      config.token = payload.token;
      updated = true;
    }
    return updated;
  } catch {
    return false;
  }
}

function resolveWsBase(): string | null {
  const wsValue = config.wsUrl.trim();
  if (wsValue) {
    return wsValue;
  }
  const apiValue = config.apiUrl.trim();
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

function resolveApiBase(): URL | null {
  const apiValue = config.apiUrl.trim();
  if (apiValue) {
    try {
      return new URL(apiValue);
    } catch {
      return null;
    }
  }
  const wsValue = config.wsUrl.trim();
  if (!wsValue) {
    return null;
  }
  try {
    const wsUrl = new URL(wsValue);
    const protocol = wsUrl.protocol === "wss:" ? "https:" : "http:";
    return new URL(`${protocol}//${wsUrl.host}`);
  } catch {
    return null;
  }
}

function buildApiUrl(path: string): string | null {
  const base = resolveApiBase();
  if (!base) {
    return null;
  }
  const url = new URL(path, base);
  const token = config.token.trim();
  if (token) {
    url.searchParams.set("token", token);
  }
  return url.toString();
}

function buildWsUrl(): string | null {
  const base = resolveWsBase();
  if (!base) {
    return null;
  }
  const url = new URL(base);
  url.searchParams.set("session", normalizedSessionValue(config.session));
  url.searchParams.set("encoding", config.encoding || "binary");
  const token = config.token.trim();
  if (token) {
    url.searchParams.set("token", token);
  }
  return url.toString();
}

function setSessionsNotice(message: string) {
  sessionListEl.replaceChildren();
  sessionEmptyEl.textContent = message;
  sessionEmptyEl.hidden = false;
  latestSessions = null;
}

function selectSession(targetId: string) {
  if (normalizedSessionValue(config.session) === targetId) {
    return;
  }
  config.session = targetId;
  if (socket) {
    void connect();
  }
  if (latestSessions) {
    renderSessions(latestSessions);
  }
}

function renderSessions(payload: SessionsResponse) {
  latestSessions = payload;
  sessionListEl.replaceChildren();
  const sessions = payload.sessions ?? [];
  const selectedValue = normalizedSessionValue(config.session);
  const selectedId =
    selectedValue === "active" ? payload.active ?? null : selectedValue;

  if (sessions.length === 0) {
    setSessionsNotice("No sessions");
    return;
  }

  sessionEmptyEl.hidden = true;

  sessions.forEach((session, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "session-item";
    button.dataset.sessionId = session.id;
    button.setAttribute("role", "option");

    if (selectedId && session.id === selectedId) {
      button.classList.add("session-item--selected");
      button.setAttribute("aria-current", "true");
    }

    // Status dot
    const dot = document.createElement("span");
    dot.className = "session-item__dot";
    if (session.running) {
      dot.classList.add("session-item__dot--running");
    }

    // Session ID
    const id = document.createElement("span");
    id.className = "session-item__id";
    id.textContent = session.id;

    button.appendChild(dot);
    button.appendChild(id);

    // Keyboard shortcut hint (1-9)
    if (index < 9) {
      const shortcut = document.createElement("span");
      shortcut.className = "session-item__shortcut";
      shortcut.textContent = `\u2318${index + 1}`;
      button.appendChild(shortcut);
    }

    button.addEventListener("click", () => selectSession(session.id));
    sessionListEl.appendChild(button);
  });
}

async function refreshSessions() {
  if (sessionsLoading) {
    return;
  }
  const url = buildApiUrl("/api/v1/sessions");
  if (!url) {
    setSessionsNotice("Waiting for daemon...");
    return;
  }
  sessionsLoading = true;
  sessionListEl.setAttribute("aria-busy", "true");
  if (sessionsRefreshBtn) {
    sessionsRefreshBtn.disabled = true;
  }
  try {
    const resp = await fetch(url);
    if (!resp.ok) {
      setSessionsNotice(`Error (${resp.status})`);
      return;
    }
    const payload = (await resp.json()) as SessionsResponse;
    renderSessions(payload);
  } catch {
    setSessionsNotice("Failed to load");
  } finally {
    sessionsLoading = false;
    sessionListEl.setAttribute("aria-busy", "false");
    if (sessionsRefreshBtn) {
      sessionsRefreshBtn.disabled = false;
    }
  }
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
        closedNoticeSent = false;
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
      break;
    case "error":
      break;
    case "closed":
      showTerminationNotice();
      refreshSessions();
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

async function connect() {
  const hydrated = await hydrateLocalState();
  if (hydrated) {
    // State loaded
  }
  void refreshSessions();
  const wsUrl = buildWsUrl();
  if (!wsUrl) {
    setStatus("No daemon", false);
    return;
  }
  disconnect("reconnect");
  closedNoticeSent = false;
  setStatus("Connecting...", false);

  const ws = new WebSocket(wsUrl);
  const state = { reason: null as DisconnectReason | null };
  socket = ws;
  socketState = state;
  ws.binaryType = "arraybuffer";

  ws.addEventListener("open", () => {
    if (socket !== ws) {
      return;
    }
    setStatus("Connected", true);
    connectBtn.textContent = "\u00D7";
    connectBtn.title = "Disconnect";
    startPeriodicRefresh();
  });

  ws.addEventListener("message", (event) => {
    if (socket !== ws) {
      return;
    }
    if (typeof event.data === "string") {
      handleTextEvent(event.data);
      return;
    }
    if (event.data instanceof ArrayBuffer) {
      handleBinaryEvent(event.data);
    }
  });

  ws.addEventListener("close", () => {
    if (socket !== ws) {
      return;
    }
    const reason = state.reason ?? "server";
    socket = null;
    socketState = null;
    finalizeDisconnect(reason);
  });

  ws.addEventListener("error", () => {
    if (socket !== ws) {
      return;
    }
    disconnect("error");
  });
}

connectBtn.addEventListener("click", () => {
  if (socket) {
    disconnect("manual");
  } else {
    void connect();
  }
});

sessionsRefreshBtn?.addEventListener("click", () => {
  void refreshSessions();
});

// Keyboard shortcuts: Cmd/Ctrl + 1-9 to switch sessions
document.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key >= "1" && e.key <= "9") {
    const index = parseInt(e.key, 10) - 1;
    const sessions = latestSessions?.sessions ?? [];
    if (index < sessions.length) {
      e.preventDefault();
      selectSession(sessions[index].id);
    }
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
    // State loaded
  }
  void refreshSessions();
}

void init();
