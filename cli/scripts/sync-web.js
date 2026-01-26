#!/usr/bin/env node

/**
 * sync-web.js - Copies the root web/ directory into the npm package folder.
 *
 * This runs before packing/publishing so the live preview gateway ships with the CLI.
 */

const fs = require("fs");
const path = require("path");

const SRC_DIR = path.join(__dirname, "..", "..", "web");
const DEST_DIR = path.join(__dirname, "..", "web");

function copyDir(src, dest) {
  if (!fs.existsSync(src)) {
    throw new Error(`Source directory not found: ${src}`);
  }

  fs.mkdirSync(dest, { recursive: true });
  const entries = fs.readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else if (entry.isSymbolicLink()) {
      const link = fs.readlinkSync(srcPath);
      fs.symlinkSync(link, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

function main() {
  if (!fs.existsSync(SRC_DIR)) {
    console.warn(`Web directory not found at ${SRC_DIR}. Skipping.`);
    return;
  }

  if (fs.existsSync(DEST_DIR)) {
    fs.rmSync(DEST_DIR, { recursive: true, force: true });
  }

  copyDir(SRC_DIR, DEST_DIR);
  console.log(`Synced ${SRC_DIR} -> ${DEST_DIR}`);
}

main();
