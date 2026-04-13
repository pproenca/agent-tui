export const MISSING_WS_ENDPOINT_MESSAGE =
  "Missing authenticated websocket endpoint. Open this UI from the daemon-provided live preview URL.";

export const INVALID_WS_ENDPOINT_MESSAGE =
  "Invalid websocket endpoint in URL. Reopen the live preview from the daemon.";

export type WsEndpointResolution = {
  endpoint: string | null;
  error: string | null;
};

export function resolveWsEndpoint(wsUrl: string): WsEndpointResolution {
  const trimmed = wsUrl.trim();
  if (!trimmed) {
    return {
      endpoint: null,
      error: MISSING_WS_ENDPOINT_MESSAGE,
    };
  }

  try {
    return {
      endpoint: new URL(trimmed).toString(),
      error: null,
    };
  } catch {
    return {
      endpoint: null,
      error: INVALID_WS_ENDPOINT_MESSAGE,
    };
  }
}
