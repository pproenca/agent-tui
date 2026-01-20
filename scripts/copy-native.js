#!/usr/bin/env node

/**
 * copy-native.js - Copies the locally built binary to bin/ with platform-specific naming
 *
 * This script is used after running `cargo build --release` to copy the built
 * binary to the bin/ directory with the correct platform/arch naming convention.
 */

const fs = require('fs');
const path = require('path');

const CLI_DIR = path.join(__dirname, '..', 'cli');
const BIN_DIR = path.join(__dirname, '..', 'bin');

function getPlatformArch() {
    const platform = process.platform;
    const arch = process.arch;

    const platformMap = {
        darwin: 'darwin',
        linux: 'linux',
        win32: 'win32',
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
    const binaryName =
        process.platform === 'win32' ? `agent-tui-${platformArch}.exe` : `agent-tui-${platformArch}`;
    return path.join(BIN_DIR, binaryName);
}

function main() {
    const platformArch = getPlatformArch();
    const sourcePath = getSourceBinaryPath();
    const destPath = getDestBinaryPath(platformArch);

    console.log(`Platform: ${platformArch}`);
    console.log(`Source: ${sourcePath}`);
    console.log(`Destination: ${destPath}`);

    if (!fs.existsSync(sourcePath)) {
        console.error(`Binary not found at ${sourcePath}`);
        console.error('Run `npm run build` first to compile the binary');
        process.exit(1);
    }

    // Ensure bin directory exists
    if (!fs.existsSync(BIN_DIR)) {
        fs.mkdirSync(BIN_DIR, { recursive: true });
    }

    // Copy the binary
    fs.copyFileSync(sourcePath, destPath);

    // Make executable on Unix
    if (process.platform !== 'win32') {
        fs.chmodSync(destPath, 0o755);
    }

    const stats = fs.statSync(destPath);
    const sizeMB = (stats.size / (1024 * 1024)).toFixed(2);

    console.log(`Copied ${sizeMB} MB to ${destPath}`);
}

main();
