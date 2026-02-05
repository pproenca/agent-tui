import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { createElement, Plus, X, RefreshCw, type IconNode } from "lucide";

const ICON_ATTRS = { width: 16, height: 16, "stroke-width": 2 };
const RPC_TIMEOUT_MS = 5000;
const REFRESH_INTERVAL_MS = 5000;

function setButtonIcon(button: HTMLButtonElement, iconDef: IconNode): void {
  button.replaceChildren(createElement(iconDef, ICON_ATTRS));
}

const connectBtn = document.getElementById("connectBtn") as HTMLButtonElement;
const statusEl = document.getElementById("status") as HTMLDivElement;
const statusText = statusEl.querySelector(".status__text") as HTMLSpanElement;
const sessionListEl = document.getElementById("sessionList") as HTMLDivElement;
const sessionEmptyEl = document.getElementById("sessionEmpty") as HTMLDivElement;
const sessionsRefreshBtn = document.getElementById("sessionsRefresh") as
  | HTMLButtonElement
  | null;

setButtonIcon(connectBtn, Plus);
if (sessionsRefreshBtn) {
  setButtonIcon(sessionsRefreshBtn, RefreshCw);
}

const params = new URLSearchParams(window.location.search);
let decoder = new TextDecoder();
let requestId = 1;

function nextRequestId(): number {
  const id = requestId;
  requestId += 1;
  return id;
}

type ConnectionConfig = {
  wsUrl: string;
  session: string;
};

const config: ConnectionConfig = {
  wsUrl: params.get("ws") ?? "",
  session: params.get("session") ?? "active",
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
let socketState: { reason: DisconnectReason | null; streamId: number | null } | null =
  null;
let closedNoticeSent = false;
let latestSessions: SessionsResponse | null = null;
let sessionsLoading = false;
let refreshInterval: ReturnType<typeof setInterval> | null = null;

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

type RpcResponse = {
  id?: number;
  result?: any;
  error?: { code?: number; message?: string };
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
  setButtonIcon(connectBtn, Plus);
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

function buildWsEndpoint(): string | null {
  const wsValue = config.wsUrl.trim();
  if (wsValue) {
    try {
      return new URL(wsValue).toString();
    } catch {
      return null;
    }
  }

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/ws`;
}

async function rpcCall(method: string, params?: Record<string, unknown>): Promise<any> {
  const endpoint = buildWsEndpoint();
  if (!endpoint) {
    throw new Error("missing websocket endpoint");
  }

  const id = nextRequestId();
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(endpoint);
    let settled = false;

    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      ws.close();
      reject(new Error("rpc timeout"));
    }, RPC_TIMEOUT_MS);

    const finish = (fn: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      ws.close();
      fn();
    };

    ws.addEventListener("open", () => {
      const payload: any = {
        jsonrpc: "2.0",
        id,
        method,
      };
      if (params && Object.keys(params).length > 0) {
        payload.params = params;
      }
      ws.send(JSON.stringify(payload));
    });

    ws.addEventListener("message", (event) => {
      if (typeof event.data !== "string") {
        return;
      }
      let response: RpcResponse;
      try {
        response = JSON.parse(event.data) as RpcResponse;
      } catch {
        return;
      }
      if (response.id !== id) {
        return;
      }
      if (response.error) {
        finish(() => reject(new Error(response.error?.message ?? "rpc error")));
        return;
      }
      finish(() => resolve(response.result ?? null));
    });

    ws.addEventListener("error", () => {
      finish(() => reject(new Error("websocket error")));
    });

    ws.addEventListener("close", () => {
      if (!settled) {
        finish(() => reject(new Error("websocket closed before rpc response")));
      }
    });
  });
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

    const dot = document.createElement("span");
    dot.className = "session-item__dot";
    if (session.running) {
      dot.classList.add("session-item__dot--running");
    }

    const id = document.createElement("span");
    id.className = "session-item__id";
    id.textContent = session.id;

    button.appendChild(dot);
    button.appendChild(id);

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
  const endpoint = buildWsEndpoint();
  if (!endpoint) {
    setSessionsNotice("Waiting for daemon...");
    return;
  }

  sessionsLoading = true;
  sessionListEl.setAttribute("aria-busy", "true");
  if (sessionsRefreshBtn) {
    sessionsRefreshBtn.disabled = true;
  }

  try {
    const result = (await rpcCall("sessions")) as {
      sessions?: SessionInfo[];
      active_session?: string | null;
    };
    renderSessions({
      sessions: Array.isArray(result.sessions) ? result.sessions : [],
      active: result.active_session ?? null,
    });
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

function handleStreamPayload(payload: any) {
  if (!payload || typeof payload !== "object") {
    return;
  }

  switch (payload.event) {
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
    case "closed":
      showTerminationNotice();
      void refreshSessions();
      break;
    default:
      break;
  }
}

async function connect() {
  void refreshSessions();
  const endpoint = buildWsEndpoint();
  if (!endpoint) {
    setStatus("No daemon", false);
    return;
  }

  disconnect("reconnect");
  closedNoticeSent = false;
  setStatus("Connecting...", false);

  const ws = new WebSocket(endpoint);
  const streamId = nextRequestId();
  const state = {
    reason: null as DisconnectReason | null,
    streamId,
  };
  socket = ws;
  socketState = state;
  decoder = new TextDecoder();

  ws.addEventListener("open", () => {
    if (socket !== ws || !socketState) {
      return;
    }

    ws.send(
      JSON.stringify({
        jsonrpc: "2.0",
        id: socketState.streamId,
        method: "live_preview_stream",
        params: { session: normalizedSessionValue(config.session) },
      }),
    );

    setStatus("Connected", true);
    setButtonIcon(connectBtn, X);
    connectBtn.title = "Disconnect";
    startPeriodicRefresh();
  });

  ws.addEventListener("message", (event) => {
    if (socket !== ws || typeof event.data !== "string" || !socketState) {
      return;
    }

    let response: RpcResponse;
    try {
      response = JSON.parse(event.data) as RpcResponse;
    } catch {
      return;
    }

    if (response.id !== socketState.streamId) {
      return;
    }

    if (response.error) {
      setStatus(`Error: ${response.error.message ?? "rpc error"}`, false);
      disconnect("error");
      return;
    }

    handleStreamPayload(response.result);
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
  const auto = params.get("auto");
  if (auto === "1" || auto === "true") {
    void connect();
    return;
  }
  void refreshSessions();
}

void init();
