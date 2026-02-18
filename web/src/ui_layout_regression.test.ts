import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SRC_DIR = dirname(fileURLToPath(import.meta.url));

function readSrcFile(name: string): string {
  return readFileSync(join(SRC_DIR, name), "utf8");
}

describe("live preview layout hooks", () => {
  test("keeps sidebar and timeline hooks in markup", () => {
    const html = readSrcFile("index.html");
    expect(html).toContain('class="sidebar__header"');
    expect(html).toContain('id="sessionCount"');
    expect(html).toContain('id="commandTimeline"');
  });

  test("keeps terminal/timeline layout resilient and visible", () => {
    const css = readSrcFile("styles.css");
    expect(css).toContain("grid-template-rows: minmax(0, 1fr) auto;");
    expect(css).toContain(".terminal {");
    expect(css).toContain("min-height: 0;");
    expect(css).toContain("grid-template-areas:");
    expect(css).toContain('"main"');
    expect(css).toContain('"sidebar"');
  });

  test("keeps session card hierarchy classes in stylesheet", () => {
    const css = readSrcFile("styles.css");
    expect(css).toContain(".session-item__status");
    expect(css).toContain(".session-item__fact");
    expect(css).toContain(".session-item__meta");
  });

  test("keeps touch-safe, labeled sidebar controls", () => {
    const html = readSrcFile("index.html");
    expect(html).toContain('id="connectBtnLabel"');
    expect(html).toContain('id="sessionsRefreshLabel"');

    const css = readSrcFile("styles.css");
    expect(css).toContain(".button__label");
    expect(css).toContain(".button--icon");
    expect(css).toContain("min-height: var(--control-height);");
  });

  test("stacks sidebar controls and wraps card metadata on narrow screens", () => {
    const css = readSrcFile("styles.css");
    expect(css).toContain("@media (max-width: 560px)");
    expect(css).toContain(".sidebar__status {\n    grid-template-columns: 1fr;");
    expect(css).toContain(
      ".sidebar__section-header > .button--icon {\n    grid-column: 1;\n    grid-row: auto;\n    justify-self: stretch;",
    );
    expect(css).toContain(".session-list {\n  display: flex;\n  flex-direction: column;\n  gap: var(--space-2);\n  overflow-y: auto;\n  overflow-x: hidden;");
    expect(css).toContain(".session-item__meta {\n  display: flex;\n  align-items: center;\n  justify-content: space-between;\n  gap: var(--space-2);\n  flex-wrap: wrap;");
    expect(css).toContain(".session-item__detail {\n  display: none;\n  color: var(--muted);\n  font-family: \"IBM Plex Mono\", monospace;\n  font-size: 10px;\n  line-height: 1.1;\n  white-space: nowrap;\n  max-width: 100%;");
    expect(css).toContain(".session-item__detail {\n    margin-left: 0;\n    white-space: normal;\n    overflow-wrap: anywhere;");
  });

  test("keeps listbox option semantics via aria-selected", () => {
    const app = readSrcFile("app.ts");
    expect(app).toContain('button.setAttribute("aria-selected", card.selected ? "true" : "false")');
    expect(app).not.toContain('button.setAttribute("aria-current", "true")');
  });

  test("keeps live preview stream size authoritative during window resize", () => {
    const app = readSrcFile("app.ts");
    expect(app).toContain("let livePreviewTerminalSize");
    expect(app).toContain('window.addEventListener("resize", () => {');
    expect(app).toContain("if (livePreviewTerminalSize) {");
    expect(app).toContain("livePreviewTerminalSize = { cols, rows };");
  });

  test("emits resize rpc when viewport changes during live preview", () => {
    const app = readSrcFile("app.ts");
    expect(app).toContain('const LIVE_PREVIEW_RESIZE_DEBOUNCE_MS');
    expect(app).toContain('rpcCall("resize", {');
    expect(app).toContain("session: connectedTerminalSessionId");
    expect(app).toContain("fitAddon.proposeDimensions()");
  });
});
