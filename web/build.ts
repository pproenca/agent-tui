import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL(".", import.meta.url));
const srcDir = join(root, "src");
const publicDir = join(root, "public");

await mkdir(publicDir, { recursive: true });

const result = await Bun.build({
  entrypoints: [join(srcDir, "app.ts")],
  outdir: publicDir,
  target: "browser",
  minify: false,
  sourcemap: "inline",
});

if (!result.success) {
  for (const message of result.logs) {
    console.error(message);
  }
  process.exit(1);
}

await Bun.write(join(publicDir, "index.html"), Bun.file(join(srcDir, "index.html")));
await Bun.write(join(publicDir, "styles.css"), Bun.file(join(srcDir, "styles.css")));
await Bun.write(
  join(publicDir, "xterm.css"),
  Bun.file(join(root, "node_modules/xterm/css/xterm.css")),
);

console.log("Built web UI to", publicDir);
