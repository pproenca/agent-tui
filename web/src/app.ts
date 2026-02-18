import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { createElement, Plus, RefreshCw, X, type IconNode } from "lucide";

import {
  buildSessionCards,
  decideTerminalSyncAction,
  normalizedSessionValue,
  reduceSessionsFeedState,
  reduceTerminalStreamState,
  shouldPromoteSelectionToActive,
  shouldAutoConnect,
  sortSessionsForFlightdeck,
  type TerminalDisconnectReason,
  type SessionInfo,
  type SessionsFeedState,
  type SessionsResponse,
  type TerminalStreamState,
} from "./session_view_model";

const ICON_ATTRS = { width: 16, height: 16, "stroke-width": 2 };
const RPC_TIMEOUT_MS = 5000;
const POLL_REFRESH_INTERVAL_MS = 5000;
const FLIGHTDECK_STREAM_INTERVAL_MS = 1000;

function setButtonIcon(button: HTMLButtonElement, iconDef: IconNode): void {
  button.replaceChildren(createElement(iconDef, ICON_ATTRS));
}

const connectBtn = document.getElementById("connectBtn") as HTMLButtonElement;
const statusEl = document.getElementById("status") as HTMLDivElement;
const statusText = statusEl.querySelector(".status__text") as HTMLSpanElement;
const sessionListEl = document.getElementById("sessionList") as HTMLDivElement;
const sessionEmptyEl = document.getElementById("sessionEmpty") as HTMLDivElement;
const sessionCountEl = document.getElementById("sessionCount") as HTMLSpanElement | null;
const sessionsRefreshBtn = document.getElementById("sessionsRefresh") as
  | HTMLButtonElement
  | null;
const sessionsModeBadgeEl = document.getElementById("sessionsModeBadge") as
  | HTMLSpanElement
  | null;

setButtonIcon(connectBtn, Plus);
if (sessionsRefreshBtn) {
  setButtonIcon(sessionsRefreshBtn, RefreshCw);
}

const params = new URLSearchParams(window.location.search);
const autoConnectEnabled = shouldAutoConnect(params.get("auto"));
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

type DisconnectReason = TerminalDisconnectReason;
type RpcResponse = {
  id?: number;
  result?: any;
  error?: { code?: number; message?: string };
};

let terminalSocket: WebSocket | null = null;
let terminalSocketState: { reason: DisconnectReason | null; streamId: number | null } | null =
  null;
let connectedTerminalSessionId: string | null = null;
let lastTerminalDisconnectReason: DisconnectReason | null = null;
let terminalState: TerminalStreamState = {
  sessionId: normalizedSessionValue(config.session),
  closedNoticeSent: false,
  closedNoticeCount: 0,
};

let sessionsSocket: WebSocket | null = null;
let sessionsSocketState: { reason: "manual" | "error" | "server" | null; streamId: number } | null =
  null;
let sessionsLoading = false;
let latestSessions: SessionsResponse | null = null;
let pollInterval: ReturnType<typeof setInterval> | null = null;
let sessionsFeedState: SessionsFeedState = {
  mode: "stream",
  degraded: false,
  selectedSession: config.session,
  payload: null,
};

function setStatus(text: string, connected: boolean) {
  statusText.textContent = text;
  if (connected) {
    statusEl.classList.add("status--connected");
  } else {
    statusEl.classList.remove("status--connected");
  }
}

function applySessionsFeedState(nextState: SessionsFeedState): void {
  sessionsFeedState = nextState;
  config.session = nextState.selectedSession;
  if (sessionsModeBadgeEl) {
    sessionsModeBadgeEl.hidden = !nextState.degraded;
    sessionsModeBadgeEl.textContent = nextState.degraded
      ? "Polling fallback"
      : "Live stream";
  }
}

function setSessionsNotice(message: string) {
  sessionListEl.replaceChildren();
  sessionEmptyEl.textContent = message;
  sessionEmptyEl.hidden = false;
  if (sessionCountEl) {
    sessionCountEl.textContent = "0";
  }
  latestSessions = null;
}

function renderSessions(payload: SessionsResponse) {
  latestSessions = payload;
  sessionListEl.replaceChildren();

  if ((payload.sessions ?? []).length === 0) {
    setSessionsNotice("No sessions");
    return;
  }

  sessionEmptyEl.hidden = true;
  const cards = buildSessionCards(payload, config.session);
  if (sessionCountEl) {
    sessionCountEl.textContent = String(cards.length);
  }
  cards.forEach((card, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "session-item";
    button.dataset.sessionId = card.id;
    button.setAttribute("role", "option");
    if (card.selected) {
      button.classList.add("session-item--selected");
      button.setAttribute("aria-current", "true");
    }

    const top = document.createElement("div");
    top.className = "session-item__top";

    const status = document.createElement("span");
    status.className = "session-item__status";

    const dot = document.createElement("span");
    dot.className = "session-item__dot";
    if (card.running) {
      dot.classList.add("session-item__dot--running");
    }
    status.appendChild(dot);

    const statusLabel = document.createElement("span");
    statusLabel.className = "session-item__status-label";
    statusLabel.textContent = card.statusLabel;
    status.appendChild(statusLabel);

    top.appendChild(status);

    const id = document.createElement("div");
    id.className = "session-item__id";
    id.textContent = card.id;

    if (index < 9) {
      const shortcut = document.createElement("span");
      shortcut.className = "session-item__shortcut";
      shortcut.textContent = `\u2318${index + 1}`;
      top.appendChild(shortcut);
    }

    const command = document.createElement("div");
    command.className = "session-item__command";
    command.textContent = card.command;

    const meta = document.createElement("div");
    meta.className = "session-item__facts";
    card.facts.forEach((fact) => {
      const factEl = document.createElement("span");
      factEl.className = "session-item__fact";
      factEl.textContent = fact;
      meta.appendChild(factEl);
    });

    button.appendChild(top);
    button.appendChild(id);
    button.appendChild(command);
    button.appendChild(meta);
    button.addEventListener("click", () => selectSession(card.id));
    sessionListEl.appendChild(button);
  });
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

function mapSessionsResult(result: any): SessionsResponse {
  const sessions = Array.isArray(result?.sessions)
    ? (result.sessions as SessionInfo[])
    : [];
  const active =
    typeof result?.active_session === "string" || result?.active_session === null
      ? (result.active_session as string | null)
      : null;
  return { sessions, active };
}

async function refreshSessionsViaRpc() {
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
    const result = await rpcCall("sessions");
    const payload = mapSessionsResult(result);
    applySessionsFeedState(
      reduceSessionsFeedState(sessionsFeedState, {
        type: "poll_payload",
        payload,
      }),
    );
    renderSessions(payload);
  } catch {
    if (!latestSessions) {
      setSessionsNotice("Failed to load");
    }
  } finally {
    sessionsLoading = false;
    sessionListEl.setAttribute("aria-busy", "false");
    if (sessionsRefreshBtn) {
      sessionsRefreshBtn.disabled = false;
    }
  }
}

function stopPollingFallback() {
  if (!pollInterval) {
    return;
  }
  clearInterval(pollInterval);
  pollInterval = null;
}

function startPollingFallback() {
  if (pollInterval) {
    return;
  }
  void refreshSessionsViaRpc();
  pollInterval = setInterval(() => {
    void refreshSessionsViaRpc();
  }, POLL_REFRESH_INTERVAL_MS);
}

function activatePollingFallback() {
  applySessionsFeedState(reduceSessionsFeedState(sessionsFeedState, { type: "stream_failure" }));
  startPollingFallback();
}

function stopSessionsFeed() {
  if (sessionsSocket && sessionsSocketState) {
    sessionsSocketState.reason = "manual";
    sessionsSocket.close();
  }
  sessionsSocket = null;
  sessionsSocketState = null;
}

function handleSessionsStreamResult(result: any): void {
  if (!result || typeof result !== "object") {
    return;
  }
  if (result.event !== "ready" && result.event !== "sessions") {
    return;
  }

  const payload = mapSessionsResult({
    sessions: result.sessions,
    active_session: result.active_session ?? null,
  });
  applySessionsFeedState(
    reduceSessionsFeedState(sessionsFeedState, {
      type: "stream_payload",
      payload,
    }),
  );
  renderSessions(payload);

  const terminalSync = decideTerminalSyncAction(config.session, payload, {
    terminalConnected: terminalSocket !== null,
    connectedSessionId: connectedTerminalSessionId,
    autoConnect: autoConnectEnabled,
    lastDisconnectReason: lastTerminalDisconnectReason,
  });
  if (terminalSync === "connect" || terminalSync === "reconnect") {
    void connect();
  }
}

function startSessionsFeed() {
  stopSessionsFeed();

  const endpoint = buildWsEndpoint();
  if (!endpoint) {
    activatePollingFallback();
    setSessionsNotice("Waiting for daemon...");
    return;
  }

  const ws = new WebSocket(endpoint);
  const streamId = nextRequestId();
  const state = {
    reason: null as "manual" | "error" | "server" | null,
    streamId,
  };
  sessionsSocket = ws;
  sessionsSocketState = state;

  ws.addEventListener("open", () => {
    if (sessionsSocket !== ws || !sessionsSocketState) {
      return;
    }
    const payload = {
      jsonrpc: "2.0",
      id: sessionsSocketState.streamId,
      method: "flightdeck_stream",
      params: {
        interval_ms: FLIGHTDECK_STREAM_INTERVAL_MS,
      },
    };
    ws.send(JSON.stringify(payload));
    stopPollingFallback();
    applySessionsFeedState(reduceSessionsFeedState(sessionsFeedState, { type: "stream_recovered" }));
  });

  ws.addEventListener("message", (event) => {
    if (sessionsSocket !== ws || !sessionsSocketState || typeof event.data !== "string") {
      return;
    }

    let response: RpcResponse;
    try {
      response = JSON.parse(event.data) as RpcResponse;
    } catch {
      return;
    }

    if (response.id !== sessionsSocketState.streamId) {
      return;
    }
    if (response.error) {
      state.reason = "error";
      activatePollingFallback();
      ws.close();
      return;
    }

    handleSessionsStreamResult(response.result);
  });

  ws.addEventListener("close", () => {
    if (sessionsSocket !== ws) {
      return;
    }
    const reason = state.reason ?? "server";
    sessionsSocket = null;
    sessionsSocketState = null;
    if (reason !== "manual") {
      activatePollingFallback();
    }
  });

  ws.addEventListener("error", () => {
    if (sessionsSocket !== ws) {
      return;
    }
    state.reason = "error";
    activatePollingFallback();
    ws.close();
  });
}

function livePreviewStreamParams(): Record<string, string> | undefined {
  const session = normalizedSessionValue(config.session);
  if (session === "active") {
    return undefined;
  }
  return { session };
}

function showTerminationNotice() {
  const next = reduceTerminalStreamState(terminalState, { type: "stream_closed" });
  const shouldWrite = next.closedNoticeCount > terminalState.closedNoticeCount;
  terminalState = next;
  if (shouldWrite) {
    term.write("\r\n\x1b[90m[session terminated]\x1b[0m\r\n");
  }
}

function finalizeTerminalDisconnect(reason: DisconnectReason) {
  connectedTerminalSessionId = null;
  lastTerminalDisconnectReason = reason;
  setButtonIcon(connectBtn, Plus);
  connectBtn.title = "Connect";
  setStatus("Disconnected", false);
  if (reason === "server") {
    showTerminationNotice();
  }
}

function disconnectTerminal(reason: DisconnectReason = "manual") {
  if (!terminalSocket) {
    finalizeTerminalDisconnect(reason);
    return;
  }
  if (terminalSocketState) {
    terminalSocketState.reason = reason;
  }
  terminalSocket.close();
}

function decodeBase64(data: string): string {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return decoder.decode(bytes, { stream: true });
}

function handleTerminalPayload(payload: any) {
  if (!payload || typeof payload !== "object") {
    return;
  }

  switch (payload.event) {
    case "ready":
      if (typeof payload.session_id === "string") {
        connectedTerminalSessionId = payload.session_id;
      }
      if (payload.cols && payload.rows) {
        term.resize(payload.cols, payload.rows);
      }
      break;
    case "init":
      if (payload.init) {
        terminalState = reduceTerminalStreamState(terminalState, { type: "reset_stream" });
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
    case "closed":
      showTerminationNotice();
      if (sessionsFeedState.mode === "poll") {
        void refreshSessionsViaRpc();
      }
      break;
    default:
      break;
  }
}

async function connect() {
  const endpoint = buildWsEndpoint();
  if (!endpoint) {
    setStatus("No daemon", false);
    return;
  }

  disconnectTerminal("reconnect");
  lastTerminalDisconnectReason = null;
  connectedTerminalSessionId = null;
  terminalState = reduceTerminalStreamState(terminalState, { type: "reset_stream" });
  setStatus("Connecting...", false);

  const ws = new WebSocket(endpoint);
  const streamId = nextRequestId();
  const state = {
    reason: null as DisconnectReason | null,
    streamId,
  };
  terminalSocket = ws;
  terminalSocketState = state;
  decoder = new TextDecoder();

  ws.addEventListener("open", () => {
    if (terminalSocket !== ws || !terminalSocketState) {
      return;
    }

    const payload: Record<string, unknown> = {
      jsonrpc: "2.0",
      id: terminalSocketState.streamId,
      method: "live_preview_stream",
    };
    const streamParams = livePreviewStreamParams();
    if (streamParams) {
      payload.params = streamParams;
    }
    ws.send(JSON.stringify(payload));

    setStatus("Connected", true);
    setButtonIcon(connectBtn, X);
    connectBtn.title = "Disconnect";
  });

  ws.addEventListener("message", (event) => {
    if (terminalSocket !== ws || typeof event.data !== "string" || !terminalSocketState) {
      return;
    }

    let response: RpcResponse;
    try {
      response = JSON.parse(event.data) as RpcResponse;
    } catch {
      return;
    }

    if (response.id !== terminalSocketState.streamId) {
      return;
    }
    if (response.error) {
      setStatus(`Error: ${response.error.message ?? "rpc error"}`, false);
      disconnectTerminal("error");
      return;
    }
    handleTerminalPayload(response.result);
  });

  ws.addEventListener("close", () => {
    if (terminalSocket !== ws) {
      return;
    }
    const reason = state.reason ?? "server";
    terminalSocket = null;
    terminalSocketState = null;
    finalizeTerminalDisconnect(reason);
  });

  ws.addEventListener("error", () => {
    if (terminalSocket !== ws) {
      return;
    }
    disconnectTerminal("error");
  });
}

function selectSession(targetId: string) {
  if (normalizedSessionValue(config.session) === targetId) {
    return;
  }
  if (shouldPromoteSelectionToActive(latestSessions, targetId)) {
    void rpcCall("attach", { session: targetId }).catch(() => {
      // Keep preview selection local even if daemon active session cannot be updated.
    });
  }
  applySessionsFeedState(
    reduceSessionsFeedState(sessionsFeedState, {
      type: "select_session",
      sessionId: targetId,
    }),
  );
  terminalState = reduceTerminalStreamState(terminalState, {
    type: "switch_session",
    sessionId: targetId,
  });
  if (terminalSocket) {
    void connect();
  }
  if (latestSessions) {
    renderSessions(latestSessions);
  }
}

connectBtn.addEventListener("click", () => {
  if (terminalSocket) {
    disconnectTerminal("manual");
  } else {
    void connect();
  }
});

sessionsRefreshBtn?.addEventListener("click", () => {
  void refreshSessionsViaRpc();
});

document.addEventListener("keydown", (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key >= "1" && event.key <= "9") {
    const index = parseInt(event.key, 10) - 1;
    const orderedSessions = sortSessionsForFlightdeck(latestSessions?.sessions ?? []);
    if (index < orderedSessions.length) {
      event.preventDefault();
      selectSession(orderedSessions[index].id);
    }
  }
});

async function init() {
  startSessionsFeed();
  if (autoConnectEnabled) {
    void connect();
  }
}

void init();
