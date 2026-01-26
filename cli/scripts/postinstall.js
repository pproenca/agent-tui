#!/usr/bin/env node

/**
 * postinstall.js - Verifies the platform-specific binary is installed via npm.
 *
 * This script runs after npm install and ensures the native binary is available
 * in the matching optional dependency package.
 */

const fs = require('fs');
const path = require('path');
const { resolveBinaryPath } = require('./platform');

async function main() {
  const resolved = resolveBinaryPath();

  if (!resolved.platformArch) {
    const message = [
      `Unsupported platform: ${process.platform}-${process.arch}`,
      'You can build from source: cargo install --git https://github.com/pproenca/agent-tui.git --path cli/crates/agent-tui',
    ];
    message.forEach((line) => console.error(line));
    process.exit(1);
  }

  if (!resolved.binPath) {
    const message = [
      `Missing platform package: ${resolved.pkgName}`,
      'Ensure the matching optional dependency is installed.',
      'You can build from source: cargo install --git https://github.com/pproenca/agent-tui.git --path cli/crates/agent-tui',
    ];
    message.forEach((line) => console.error(line));
    process.exit(1);
  }

  if (!fs.existsSync(resolved.binPath)) {
    const message = [
      `Binary not found: ${resolved.binPath}`,
      'Ensure the matching optional dependency ships the binary.',
    ];
    message.forEach((line) => console.error(line));
    process.exit(1);
  }

  if (process.platform !== 'win32') {
    fs.chmodSync(resolved.binPath, 0o755);
  }

  const relPath = path.relative(process.cwd(), resolved.binPath);
  console.log(`Installed agent-tui binary: ${relPath}`);
}

main().catch((error) => {
  console.error('Postinstall error:', error.message);
  process.exit(1);
});
