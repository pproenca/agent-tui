import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SRC_DIR = dirname(fileURLToPath(import.meta.url));

function readSrcFile(name: string): string {
  return readFileSync(join(SRC_DIR, name), "utf8");
}

describe("live preview layout hooks", () => {
  test("keeps sidebar header and session count affordance", () => {
    const html = readSrcFile("index.html");
    expect(html).toContain('class="sidebar__header"');
    expect(html).toContain('id="sessionCount"');
  });

  test("keeps session card hierarchy classes in stylesheet", () => {
    const css = readSrcFile("styles.css");
    expect(css).toContain(".session-item__status");
    expect(css).toContain(".session-item__facts");
    expect(css).toContain("grid-template-columns: clamp(");
  });

  test("keeps wide sidebar grid and metadata pill treatment", () => {
    const css = readSrcFile("styles.css");
    expect(css).toContain("grid-template-columns: clamp(260px, 24vw, 340px) minmax(0, 1fr);");
    expect(css).toContain(".session-item__fact");
    expect(css).toContain("box-shadow: var(--shadow-soft);");
  });
});
