import type { SessionInfo, SessionsResponse } from "./session_view_model";

export type RpcResponse = {
  id?: number;
  result?: unknown;
  error?: { code?: number; message?: string };
};

type RpcStreamDispatch =
  | { kind: "ignore" }
  | { kind: "error"; message: string };

export type SessionsStreamDispatch =
  | RpcStreamDispatch
  | { kind: "payload"; payload: SessionsResponse };

export type TerminalStreamEvent =
  | { type: "ready"; sessionId: string | null; cols: number | null; rows: number | null }
  | { type: "init"; init: string }
  | { type: "output"; dataB64: string }
  | { type: "resize"; cols: number; rows: number }
  | { type: "closed" }
  | { type: "heartbeat" }
  | { type: "dropped"; droppedBytes: number };

export type TerminalStreamDispatch =
  | RpcStreamDispatch
  | { kind: "payload"; payload: TerminalStreamEvent };

function ignore(): RpcStreamDispatch {
  return { kind: "ignore" };
}

function rpcErrorMessage(message: string | undefined): string {
  return message?.trim() ? message : "rpc error";
}

function asFiniteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function mapSessionsResult(result: unknown): SessionsResponse {
  const payload =
    result && typeof result === "object" ? (result as Record<string, unknown>) : {};
  const sessions = Array.isArray(payload.sessions)
    ? (payload.sessions as SessionInfo[])
    : [];
  const active =
    typeof payload.active_session === "string" || payload.active_session === null
      ? (payload.active_session as string | null)
      : null;
  return { sessions, active };
}

export function readSessionsStreamEnvelope(
  response: RpcResponse,
  streamId: number,
): SessionsStreamDispatch {
  if (response.id !== streamId) {
    return ignore();
  }
  if (response.error) {
    return {
      kind: "error",
      message: rpcErrorMessage(response.error.message),
    };
  }

  const result =
    response.result && typeof response.result === "object"
      ? (response.result as Record<string, unknown>)
      : null;
  if (!result) {
    return ignore();
  }
  if (result.event !== "ready" && result.event !== "sessions") {
    return ignore();
  }
  return {
    kind: "payload",
    payload: mapSessionsResult(result),
  };
}

export function readTerminalEvent(payload: unknown): TerminalStreamEvent | null {
  const result =
    payload && typeof payload === "object" ? (payload as Record<string, unknown>) : null;
  if (!result || typeof result.event !== "string") {
    return null;
  }

  switch (result.event) {
    case "ready":
      return {
        type: "ready",
        sessionId: typeof result.session_id === "string" ? result.session_id : null,
        cols: asFiniteNumber(result.cols),
        rows: asFiniteNumber(result.rows),
      };
    case "init":
      if (typeof result.init !== "string") {
        return null;
      }
      return { type: "init", init: result.init };
    case "output":
      if (typeof result.data_b64 !== "string") {
        return null;
      }
      return { type: "output", dataB64: result.data_b64 };
    case "resize": {
      const cols = asFiniteNumber(result.cols);
      const rows = asFiniteNumber(result.rows);
      if (cols === null || rows === null) {
        return null;
      }
      return { type: "resize", cols, rows };
    }
    case "closed":
      return { type: "closed" };
    case "heartbeat":
      return { type: "heartbeat" };
    case "dropped": {
      const droppedBytes = asFiniteNumber(result.dropped_bytes);
      if (droppedBytes === null) {
        return null;
      }
      return { type: "dropped", droppedBytes };
    }
    default:
      return null;
  }
}

export function readTerminalStreamEnvelope(
  response: RpcResponse,
  streamId: number,
): TerminalStreamDispatch {
  if (response.id !== streamId) {
    return ignore();
  }
  if (response.error) {
    return {
      kind: "error",
      message: rpcErrorMessage(response.error.message),
    };
  }
  const payload = readTerminalEvent(response.result);
  if (!payload) {
    return ignore();
  }
  return {
    kind: "payload",
    payload,
  };
}
