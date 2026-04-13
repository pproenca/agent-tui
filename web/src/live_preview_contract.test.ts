import { describe, expect, test } from "bun:test";

import {
  mapSessionsResult,
  readSessionsStreamEnvelope,
  readTerminalEvent,
  readTerminalStreamEnvelope,
  type RpcResponse,
} from "./live_preview_contract";

describe("mapSessionsResult", () => {
  test("maps daemon active_session payloads into the browser view model", () => {
    expect(
      mapSessionsResult({
        active_session: "session-2",
        sessions: [{ id: "session-2", command: "bash", pid: 42, running: true, created_at: "2026-01-01T00:00:00Z", size: { cols: 80, rows: 24 } }],
      }),
    ).toEqual({
      active: "session-2",
      sessions: [{ id: "session-2", command: "bash", pid: 42, running: true, created_at: "2026-01-01T00:00:00Z", size: { cols: 80, rows: 24 } }],
    });
  });
});

describe("readSessionsStreamEnvelope", () => {
  test("accepts ready envelopes for the active stream id", () => {
    const response: RpcResponse = {
      id: 11,
      result: {
        event: "ready",
        active_session: "alpha",
        sessions: [],
      },
    };

    expect(readSessionsStreamEnvelope(response, 11)).toEqual({
      kind: "payload",
      payload: {
        active: "alpha",
        sessions: [],
      },
    });
  });

  test("surfaces rpc errors for the active stream id", () => {
    expect(
      readSessionsStreamEnvelope(
        {
          id: 12,
          error: { code: -32001, message: "unauthorized" },
        },
        12,
      ),
    ).toEqual({
      kind: "error",
      message: "unauthorized",
    });
  });

  test("ignores envelopes for other request ids", () => {
    expect(
      readSessionsStreamEnvelope(
        {
          id: 7,
          result: { event: "ready", active_session: "alpha", sessions: [] },
        },
        8,
      ),
    ).toEqual({ kind: "ignore" });
  });
});

describe("readTerminalEvent", () => {
  test("parses live preview events emitted by the daemon", () => {
    expect(
      readTerminalEvent({ event: "ready", session_id: "alpha", cols: 120, rows: 40 }),
    ).toEqual({
      type: "ready",
      sessionId: "alpha",
      cols: 120,
      rows: 40,
    });

    expect(readTerminalEvent({ event: "init", init: "screen" })).toEqual({
      type: "init",
      init: "screen",
    });

    expect(readTerminalEvent({ event: "output", data_b64: "aGVsbG8=" })).toEqual({
      type: "output",
      dataB64: "aGVsbG8=",
    });

    expect(readTerminalEvent({ event: "resize", cols: 90, rows: 30 })).toEqual({
      type: "resize",
      cols: 90,
      rows: 30,
    });

    expect(readTerminalEvent({ event: "dropped", dropped_bytes: 64 })).toEqual({
      type: "dropped",
      droppedBytes: 64,
    });

    expect(readTerminalEvent({ event: "closed" })).toEqual({ type: "closed" });
    expect(readTerminalEvent({ event: "heartbeat" })).toEqual({ type: "heartbeat" });
  });

  test("ignores unsupported command events", () => {
    expect(
      readTerminalEvent({
        event: "command",
        kind: "type",
        value: "pwd",
      }),
    ).toBeNull();
  });
});

describe("readTerminalStreamEnvelope", () => {
  test("surfaces rpc errors and ignores stale ids", () => {
    expect(
      readTerminalStreamEnvelope(
        {
          id: 19,
          error: { code: -32000, message: "too many websocket connections" },
        },
        19,
      ),
    ).toEqual({
      kind: "error",
      message: "too many websocket connections",
    });

    expect(
      readTerminalStreamEnvelope(
        {
          id: 19,
          result: { event: "closed" },
        },
        20,
      ),
    ).toEqual({ kind: "ignore" });
  });
});
