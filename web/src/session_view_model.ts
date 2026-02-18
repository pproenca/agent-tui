export type SessionInfo = {
  id: string;
  command: string;
  pid: number;
  running: boolean;
  created_at: string;
  size: { cols: number; rows: number };
};

export type SessionsResponse = {
  active?: string | null;
  sessions: SessionInfo[];
};

export type SessionCardViewModel = {
  id: string;
  command: string;
  statusLabel: string;
  pidLabel: string;
  sizeLabel: string;
  createdLabel: string;
  facts: string[];
  detailLabel: string;
  running: boolean;
  selected: boolean;
};

export function normalizedSessionValue(value: string): string {
  const trimmed = value.trim();
  return trimmed.length === 0 ? "active" : trimmed;
}

export function resolveSelectedSessionId(
  configuredSession: string,
  payload: SessionsResponse,
): string | null {
  const selected = normalizedSessionValue(configuredSession);
  if (selected === "active") {
    return payload.active ?? null;
  }
  return selected;
}

function createdAtTimestamp(value: string): number {
  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp) ? 0 : timestamp;
}

export function sortSessionsForFlightdeck(sessions: SessionInfo[]): SessionInfo[] {
  return [...sessions].sort((a, b) => {
    if (a.running !== b.running) {
      return a.running ? -1 : 1;
    }
    const createdDelta = createdAtTimestamp(b.created_at) - createdAtTimestamp(a.created_at);
    if (createdDelta !== 0) {
      return createdDelta;
    }
    return a.id.localeCompare(b.id);
  });
}

export function formatSessionCreatedAt(createdAt: string): string {
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) {
    return "Unknown time";
  }
  return `${date.toISOString().slice(0, 16).replace("T", " ")}Z`;
}

export function shouldAutoConnect(autoParam: string | null): boolean {
  if (!autoParam) {
    return true;
  }
  const normalized = autoParam.trim().toLowerCase();
  if (normalized === "0" || normalized === "false") {
    return false;
  }
  return true;
}

export type TerminalDisconnectReason = "manual" | "reconnect" | "server" | "error";
export type TerminalSyncAction = "none" | "reconnect" | "connect";

export type TerminalSyncContext = {
  terminalConnected: boolean;
  connectedSessionId: string | null;
  autoConnect: boolean;
  lastDisconnectReason: TerminalDisconnectReason | null;
};

export function decideTerminalSyncAction(
  configuredSession: string,
  payload: SessionsResponse,
  context: TerminalSyncContext,
): TerminalSyncAction {
  if (normalizedSessionValue(configuredSession) !== "active") {
    return "none";
  }

  const activeSession = payload.active ?? null;
  if (!activeSession) {
    return "none";
  }

  if (context.terminalConnected) {
    if (!context.connectedSessionId) {
      return "none";
    }
    return context.connectedSessionId === activeSession ? "none" : "reconnect";
  }

  if (!context.autoConnect || context.lastDisconnectReason === "manual") {
    return "none";
  }

  return "connect";
}

export function buildSessionCards(
  payload: SessionsResponse,
  configuredSession: string,
): SessionCardViewModel[] {
  const selectedId = resolveSelectedSessionId(configuredSession, payload);
  return sortSessionsForFlightdeck(payload.sessions).map((session) => {
    const pidLabel = session.pid > 0 ? `pid ${session.pid}` : "pid -";
    const sizeLabel = `${session.size.cols}x${session.size.rows}`;
    const createdLabel = formatSessionCreatedAt(session.created_at);
    return {
      id: session.id,
      command: session.command || "(unknown)",
      statusLabel: session.running ? "running" : "stopped",
      pidLabel,
      sizeLabel,
      createdLabel,
      facts: [pidLabel],
      detailLabel: `${sizeLabel} · ${createdLabel}`,
      running: session.running,
      selected: selectedId === session.id,
    };
  });
}

export function shouldPromoteSelectionToActive(
  payload: SessionsResponse | null,
  selectedSessionId: string,
): boolean {
  if (!payload) {
    return false;
  }
  const selected = payload.sessions.find((session) => session.id === selectedSessionId);
  return selected?.running ?? false;
}

export type TerminalStreamState = {
  sessionId: string;
  closedNoticeSent: boolean;
  closedNoticeCount: number;
};

export type TerminalStreamAction =
  | { type: "stream_closed" }
  | { type: "switch_session"; sessionId: string }
  | { type: "reset_stream" };

export function reduceTerminalStreamState(
  state: TerminalStreamState,
  action: TerminalStreamAction,
): TerminalStreamState {
  switch (action.type) {
    case "stream_closed":
      if (state.closedNoticeSent) {
        return state;
      }
      return {
        ...state,
        closedNoticeSent: true,
        closedNoticeCount: state.closedNoticeCount + 1,
      };
    case "switch_session":
      if (state.sessionId === action.sessionId) {
        return state;
      }
      return {
        ...state,
        sessionId: action.sessionId,
        closedNoticeSent: false,
      };
    case "reset_stream":
      return {
        ...state,
        closedNoticeSent: false,
      };
    default:
      return state;
  }
}

export type SessionsFeedState = {
  mode: "stream" | "poll";
  degraded: boolean;
  selectedSession: string;
  payload: SessionsResponse | null;
};

export type SessionsFeedAction =
  | { type: "stream_payload"; payload: SessionsResponse }
  | { type: "poll_payload"; payload: SessionsResponse }
  | { type: "stream_failure" }
  | { type: "stream_recovered" }
  | { type: "select_session"; sessionId: string };

export function reduceSessionsFeedState(
  state: SessionsFeedState,
  action: SessionsFeedAction,
): SessionsFeedState {
  switch (action.type) {
    case "stream_payload":
      return {
        ...state,
        mode: "stream",
        degraded: false,
        payload: action.payload,
      };
    case "poll_payload":
      return {
        ...state,
        payload: action.payload,
      };
    case "stream_failure":
      return {
        ...state,
        mode: "poll",
        degraded: true,
      };
    case "stream_recovered":
      return {
        ...state,
        mode: "stream",
        degraded: false,
      };
    case "select_session":
      return {
        ...state,
        selectedSession: action.sessionId,
      };
    default:
      return state;
  }
}
