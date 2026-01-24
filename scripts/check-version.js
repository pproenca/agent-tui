#!/usr/bin/env node

/**
 * check-version.js - Validates version consistency between package.json and Cargo.toml
 *
 * Usage:
 *   node scripts/check-version.js          # Normal mode (outputs success/failure)
 *   node scripts/check-version.js --quiet  # Quiet mode (only outputs on error)
 *
 * Exit codes:
 *   0 - Versions match
 *   1 - Versions mismatch or error reading files
 */

const fs = require('fs');
const path = require('path');

const PACKAGE_JSON_PATH = path.join(__dirname, '..', 'package.json');
const CARGO_TOML_PATH = path.join(__dirname, '..', 'Cargo.toml');

function getPackageVersion() {
  const packageJson = JSON.parse(fs.readFileSync(PACKAGE_JSON_PATH, 'utf8'));
  return packageJson.version;
}

function getCargoVersion() {
  const cargoToml = fs.readFileSync(CARGO_TOML_PATH, 'utf8');
  const versionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!versionMatch) {
    throw new Error('Could not find version field in Cargo.toml');
  }
  return versionMatch[1];
}

function main() {
  const quiet = process.argv.includes('--quiet');

  try {
    const packageVersion = getPackageVersion();
    const cargoVersion = getCargoVersion();

    if (packageVersion === cargoVersion) {
      if (!quiet) {
        console.log(`Version check passed: ${packageVersion}`);
      }
      process.exit(0);
    } else {
      console.error(`ERROR: Version mismatch detected!`);
      console.error(`  package.json: ${packageVersion}`);
      console.error(`  Cargo.toml:   ${cargoVersion}`);
      console.error('');
      console.error('To fix, use: ./scripts/release.sh <version>');
      console.error('  or: just release-patch | release-minor | release-major');
      process.exit(1);
    }
  } catch (error) {
    console.error(`Version check failed: ${error.message}`);
    process.exit(1);
  }
}

main();
