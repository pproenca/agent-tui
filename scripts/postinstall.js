#!/usr/bin/env node

/**
 * postinstall.js - Downloads the correct agent-tui binary for the current platform
 *
 * This script runs after npm install and ensures the native binary is available.
 * It will:
 * 1. Check if the binary already exists (bundled in npm package)
 * 2. If not, download from GitHub releases
 */

const { execSync } = require('child_process');
const fs = require('fs');
const https = require('https');
const path = require('path');
const { createWriteStream } = require('fs');

const REPO = 'pproenca/agent-tui';
const BIN_DIR = path.join(__dirname, '..', 'bin');

function getPlatformArch() {
    const platform = process.platform;
    const arch = process.arch;

    const platformMap = {
        'darwin': 'darwin',
        'linux': 'linux'
    };

    const archMap = {
        'x64': 'x64',
        'arm64': 'arm64'
    };

    const mappedPlatform = platformMap[platform];
    const mappedArch = archMap[arch];

    if (!mappedPlatform || !mappedArch) {
        return null;
    }

    return `${mappedPlatform}-${mappedArch}`;
}

function getBinaryName(platformArch) {
    const name = `agent-tui-${platformArch}`;
    return process.platform === 'win32' ? `${name}.exe` : name;
}

function getPackageVersion() {
    const packageJson = require('../package.json');
    return packageJson.version;
}

async function downloadFile(url, dest) {
    return new Promise((resolve, reject) => {
        const follow = (url, redirectCount = 0) => {
            if (redirectCount > 5) {
                reject(new Error('Too many redirects'));
                return;
            }

            const protocol = url.startsWith('https') ? https : require('http');
            protocol.get(url, (response) => {
                if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
                    follow(response.headers.location, redirectCount + 1);
                    return;
                }

                if (response.statusCode !== 200) {
                    reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
                    return;
                }

                const file = createWriteStream(dest);
                response.pipe(file);
                file.on('finish', () => {
                    file.close();
                    resolve();
                });
                file.on('error', (err) => {
                    fs.unlink(dest, () => {});
                    reject(err);
                });
            }).on('error', reject);
        };

        follow(url);
    });
}

async function main() {
    const platformArch = getPlatformArch();

    if (!platformArch) {
        console.log(`Unsupported platform: ${process.platform}-${process.arch}`);
        console.log('You can build from source: cargo install agent-tui');
        process.exit(0);
    }

    const binaryName = getBinaryName(platformArch);
    const binaryPath = path.join(BIN_DIR, binaryName);

    // Check if binary already exists (bundled in npm package)
    if (fs.existsSync(binaryPath)) {
        console.log(`Binary already exists: ${binaryName}`);
        // Ensure it's executable
        if (process.platform !== 'win32') {
            fs.chmodSync(binaryPath, 0o755);
        }
        return;
    }

    // Download from GitHub releases
    const version = getPackageVersion();
    const tag = `v${version}`;
    const downloadUrl = `https://github.com/${REPO}/releases/download/${tag}/${binaryName}`;

    console.log(`Downloading agent-tui ${version} for ${platformArch}...`);
    console.log(`URL: ${downloadUrl}`);

    try {
        // Ensure bin directory exists
        if (!fs.existsSync(BIN_DIR)) {
            fs.mkdirSync(BIN_DIR, { recursive: true });
        }

        await downloadFile(downloadUrl, binaryPath);

        // Make executable on Unix
        if (process.platform !== 'win32') {
            fs.chmodSync(binaryPath, 0o755);
        }

        console.log(`Successfully installed agent-tui ${version}`);
    } catch (error) {
        console.error(`Failed to download binary: ${error.message}`);
        console.log('');
        console.log('Alternative installation methods:');
        console.log('  1. Install via cargo: cargo install agent-tui');
        console.log('  2. Download manually from: https://github.com/' + REPO + '/releases');
        console.log('');

        // Don't fail the install - user can still use cargo install
        process.exit(0);
    }
}

main().catch((error) => {
    console.error('Postinstall error:', error.message);
    process.exit(0);
});
