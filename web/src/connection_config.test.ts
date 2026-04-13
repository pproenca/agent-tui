import { describe, expect, test } from "bun:test";

import {
  INVALID_WS_ENDPOINT_MESSAGE,
  MISSING_WS_ENDPOINT_MESSAGE,
  resolveWsEndpoint,
} from "./connection_config";

describe("resolveWsEndpoint", () => {
  test("requires a daemon-provided authenticated websocket endpoint", () => {
    expect(resolveWsEndpoint("")).toEqual({
      endpoint: null,
      error: MISSING_WS_ENDPOINT_MESSAGE,
    });
  });

  test("rejects malformed websocket endpoints", () => {
    expect(resolveWsEndpoint("://bad endpoint")).toEqual({
      endpoint: null,
      error: INVALID_WS_ENDPOINT_MESSAGE,
    });
  });

  test("accepts explicit websocket endpoints", () => {
    expect(resolveWsEndpoint("ws://127.0.0.1:1234/ws?token=test")).toEqual({
      endpoint: "ws://127.0.0.1:1234/ws?token=test",
      error: null,
    });
  });
});
