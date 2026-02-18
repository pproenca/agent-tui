import { describe, expect, test } from "bun:test";

import {
  buildSessionCards,
  formatSessionCreatedAt,
  reduceSessionsFeedState,
  reduceTerminalStreamState,
  resolveSelectedSessionId,
  sortSessionsForFlightdeck,
  shouldAutoConnect,
  type SessionsFeedState,
  type SessionsResponse,
  type SessionInfo,
  type TerminalStreamState,
} from "./session_view_model";

const BASE_SESSIONS: SessionInfo[] = [
  {
    id: "alpha",
    command: "bash",
    pid: 111,
    running: true,
    created_at: "2026-02-18T09:00:00Z",
    size: { cols: 120, rows: 40 },
  },
  {
    id: "beta",
    command: "npm run dev",
    pid: 222,
    running: false,
    created_at: "2026-02-18T10:00:00Z",
    size: { cols: 100, rows: 30 },
  },
  {
    id: "gamma",
    command: "cargo test",
    pid: 333,
    running: true,
    created_at: "2026-02-18T11:00:00Z",
    size: { cols: 80, rows: 24 },
  },
];

describe("session selection", () => {
  test("resolves selected session from active when configured as active", () => {
    const payload: SessionsResponse = {
      active: "gamma",
      sessions: BASE_SESSIONS,
    };
    expect(resolveSelectedSessionId("active", payload)).toBe("gamma");
  });

  test("keeps explicit configured session id", () => {
    const payload: SessionsResponse = {
      active: "gamma",
      sessions: BASE_SESSIONS,
    };
    expect(resolveSelectedSessionId("beta", payload)).toBe("beta");
  });
});

describe("session ordering", () => {
  test("sorts running sessions first and newest first within groups", () => {
    const sorted = sortSessionsForFlightdeck(BASE_SESSIONS);
    expect(sorted.map((session) => session.id)).toEqual(["gamma", "alpha", "beta"]);
  });
});

describe("session formatting", () => {
  test("formats creation date consistently", () => {
    expect(formatSessionCreatedAt("2026-02-18T11:00:00Z")).toBe("2026-02-18 11:00Z");
  });

  test("returns fallback for invalid creation date", () => {
    expect(formatSessionCreatedAt("not-a-date")).toBe("Unknown time");
  });

  test("builds session cards with explicit running metadata", () => {
    const cards = buildSessionCards(
      {
        active: "alpha",
        sessions: BASE_SESSIONS,
      },
      "active",
    );
    expect(cards[0]?.statusLabel).toBe("running");
    expect(cards[2]?.statusLabel).toBe("stopped");
    expect(cards[0]?.pidLabel).toBe("pid 333");
    expect(cards[0]?.sizeLabel).toBe("80x24");
  });
});

describe("auto-connect", () => {
  test("auto-connects by default when query param is missing", () => {
    expect(shouldAutoConnect(null)).toBe(true);
  });

  test("respects explicit opt-out values", () => {
    expect(shouldAutoConnect("0")).toBe(false);
    expect(shouldAutoConnect("false")).toBe(false);
  });
});

describe("terminal reducer", () => {
  test("closed event sets notice once", () => {
    const initial: TerminalStreamState = {
      sessionId: "alpha",
      closedNoticeSent: false,
      closedNoticeCount: 0,
    };
    const once = reduceTerminalStreamState(initial, { type: "stream_closed" });
    const twice = reduceTerminalStreamState(once, { type: "stream_closed" });

    expect(once.closedNoticeSent).toBe(true);
    expect(once.closedNoticeCount).toBe(1);
    expect(twice.closedNoticeCount).toBe(1);
  });

  test("switching session resets closed notice state", () => {
    const closed: TerminalStreamState = {
      sessionId: "alpha",
      closedNoticeSent: true,
      closedNoticeCount: 1,
    };
    const switched = reduceTerminalStreamState(closed, {
      type: "switch_session",
      sessionId: "beta",
    });
    expect(switched.closedNoticeSent).toBe(false);
    expect(switched.closedNoticeCount).toBe(1);
    expect(switched.sessionId).toBe("beta");
  });
});

describe("sessions feed reducer", () => {
  test("updates sessions from stream payload and keeps mode", () => {
    const state: SessionsFeedState = {
      mode: "stream",
      degraded: false,
      selectedSession: "gamma",
      payload: null,
    };
    const next = reduceSessionsFeedState(state, {
      type: "stream_payload",
      payload: {
        active: "alpha",
        sessions: BASE_SESSIONS,
      },
    });

    expect(next.mode).toBe("stream");
    expect(next.payload?.sessions.length).toBe(3);
    expect(next.selectedSession).toBe("gamma");
  });

  test("falls back to polling on stream failure", () => {
    const state: SessionsFeedState = {
      mode: "stream",
      degraded: false,
      selectedSession: "active",
      payload: null,
    };
    const next = reduceSessionsFeedState(state, { type: "stream_failure" });
    expect(next.mode).toBe("poll");
    expect(next.degraded).toBe(true);
  });
});
