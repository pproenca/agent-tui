#!/usr/bin/env node

/**
 * sync-version.js - Synchronizes version between package.json and Cargo.toml
 *
 * This script is called automatically by npm's version lifecycle hook.
 * It reads the version from package.json and updates cli/Cargo.toml to match.
 */

const fs = require('fs');
const path = require('path');

const PACKAGE_JSON_PATH = path.join(__dirname, '..', 'package.json');
const CARGO_TOML_PATH = path.join(__dirname, '..', 'cli', 'Cargo.toml');

function getPackageVersion() {
  const packageJson = JSON.parse(fs.readFileSync(PACKAGE_JSON_PATH, 'utf8'));
  return packageJson.version;
}

function updateCargoVersion(newVersion) {
  let cargoToml = fs.readFileSync(CARGO_TOML_PATH, 'utf8');

  // Match the version line in [package] section
  // This regex handles: version = "x.y.z"
  const versionRegex = /^(version\s*=\s*")([^"]+)(")/m;

  if (!versionRegex.test(cargoToml)) {
    throw new Error('Could not find version field in Cargo.toml');
  }

  const oldVersion = cargoToml.match(versionRegex)[2];

  if (oldVersion === newVersion) {
    console.log(`Version already in sync: ${newVersion}`);
    return false;
  }

  cargoToml = cargoToml.replace(versionRegex, `$1${newVersion}$3`);
  fs.writeFileSync(CARGO_TOML_PATH, cargoToml);

  console.log(`Updated Cargo.toml version: ${oldVersion} -> ${newVersion}`);
  return true;
}

function main() {
  try {
    const version = getPackageVersion();
    console.log(`Syncing version to: ${version}`);

    const updated = updateCargoVersion(version);

    if (updated) {
      console.log('Version sync complete');
    }

    process.exit(0);
  } catch (error) {
    console.error(`Version sync failed: ${error.message}`);
    process.exit(1);
  }
}

main();
