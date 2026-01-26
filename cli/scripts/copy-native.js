#!/usr/bin/env node

/**
 * copy-native.js - Copies the locally built binary into the matching npm package.
 *
 * This script is used after running `cargo build --release` to copy the built
 * binary to cli/npm/agent-tui-<platform>/bin/agent-tui.
 */

const fs = require('fs');
const path = require('path');

const CLI_DIR = path.join(__dirname, '..');
const NPM_DIR = path.join(__dirname, '..', 'npm');

function getPlatformArch() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    darwin: 'darwin',
    linux: 'linux',
  };

  const archMap = {
    x64: 'x64',
    arm64: 'arm64',
  };

  const mappedPlatform = platformMap[platform];
  const mappedArch = archMap[arch];

  if (!mappedPlatform || !mappedArch) {
    throw new Error(`Unsupported platform: ${platform}-${arch}`);
  }

  return `${mappedPlatform}-${mappedArch}`;
}

function getSourceBinaryPath() {
  const binaryName = process.platform === 'win32' ? 'agent-tui.exe' : 'agent-tui';
  return path.join(CLI_DIR, 'target', 'release', binaryName);
}

function getDestBinaryPath(platformArch) {
  const pkgDir = path.join(NPM_DIR, `agent-tui-${platformArch}`);
  const binDir = path.join(pkgDir, 'bin');
  const binaryName = process.platform === 'win32' ? 'agent-tui.exe' : 'agent-tui';
  return { binDir, destPath: path.join(binDir, binaryName), pkgDir };
}

function main() {
  const platformArch = getPlatformArch();
  const sourcePath = getSourceBinaryPath();
  const { binDir, destPath, pkgDir } = getDestBinaryPath(platformArch);

  console.log(`Platform: ${platformArch}`);
  console.log(`Source: ${sourcePath}`);
  console.log(`Destination: ${destPath}`);

  if (!fs.existsSync(sourcePath)) {
    console.error(`Binary not found at ${sourcePath}`);
    console.error('Run `npm run build:native` first to compile the binary');
    process.exit(1);
  }

  if (!fs.existsSync(pkgDir)) {
    console.error(`Platform package not found at ${pkgDir}`);
    console.error('Ensure the npm platform packages exist under cli/npm.');
    process.exit(1);
  }

  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  fs.copyFileSync(sourcePath, destPath);

  if (process.platform !== 'win32') {
    fs.chmodSync(destPath, 0o755);
  }

  const stats = fs.statSync(destPath);
  const sizeMB = (stats.size / (1024 * 1024)).toFixed(2);

  console.log(`Copied ${sizeMB} MB to ${destPath}`);
}

main();
