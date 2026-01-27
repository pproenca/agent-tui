import { spawnSync } from "child_process";
import crypto from "crypto";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const HERE = fileURLToPath(new URL(".", import.meta.url));
const ROOT = path.resolve(HERE, "..");
const PROGRAM_NAME = "xtask";

function main() {
  try {
    run(process.argv.slice(2));
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(`${PROGRAM_NAME}: error: ${message}`);
    process.exit(1);
  }
}

function run(args: string[]) {
  const [command, ...rest] = args;
  if (!command) {
    printUsage();
    process.exit(1);
  }

  switch (command) {
    case "version":
      return versionCommand(rest);
    case "release":
      return releaseCommand(rest);
    case "ci":
      return ci();
    case "pre-push":
      return ci();
    case "architecture":
      return architectureCommand(rest);
    case "dist":
      return distCommand(rest);
    case "--help":
    case "-h":
      printUsage();
      return;
    default:
      throw new Error(`Unknown command: ${command}`);
  }
}

function printUsage() {
  console.log(
    [
      "Usage:",
      "  xtask version <check|current|assert-tag|assert-input>",
      "  xtask release <version|major|minor|patch> [--yes]",
      "  xtask ci",
      "  xtask architecture check",
      "  xtask dist <release|verify|npm>",
    ].join("\n"),
  );
}

function versionCommand(args: string[]) {
  const [subcommand, ...rest] = args;
  if (!subcommand) {
    throw new Error("Missing version subcommand");
  }

  switch (subcommand) {
    case "check": {
      const { flags } = parseFlags(rest);
      const quiet = Boolean(flags.get("quiet"));
      return versionCheck(quiet);
    }
    case "current":
      console.log(readPackageVersion(packageJsonPath(ROOT)));
      return;
    case "assert-tag": {
      const tag = rest[0];
      if (!tag) {
        throw new Error("Missing tag argument");
      }
      return assertTag(tag);
    }
    case "assert-input": {
      const version = rest[0];
      if (!version) {
        throw new Error("Missing version argument");
      }
      return assertInput(version);
    }
    default:
      throw new Error(`Unknown version subcommand: ${subcommand}`);
  }
}

function releaseCommand(args: string[]) {
  const { flags, positionals } = parseFlags(args);
  const confirm = resolveConfirm(flags);
  if (flags.size > 0 && flags.get("yes") !== true) {
    throw new Error("Only --yes is supported for release.");
  }
  if (positionals.length !== 1) {
    throw new Error("Missing release version or bump");
  }
  return release(positionals[0], { confirm });
}

function architectureCommand(args: string[]) {
  const subcommand = args[0];
  if (!subcommand) {
    throw new Error("Missing architecture subcommand");
  }
  if (subcommand !== "check") {
    throw new Error(`Unknown architecture subcommand: ${subcommand}`);
  }
  return architectureCheck();
}

function distCommand(args: string[]) {
  const [subcommand, ...rest] = args;
  if (!subcommand) {
    throw new Error("Missing dist subcommand");
  }

  switch (subcommand) {
    case "release": {
      const { flags } = parseFlags(rest);
      const input = flagString(flags, "input", "artifacts");
      const output = flagString(flags, "output", "release");
      return distRelease(pathFromRoot(input), pathFromRoot(output));
    }
    case "verify": {
      const { flags } = parseFlags(rest);
      const input = flagString(flags, "input", "artifacts");
      const kindValue = flagString(flags, "kind", "release");
      if (kindValue !== "release" && kindValue !== "npm") {
        throw new Error(`Invalid kind: ${kindValue}`);
      }
      return distVerify(pathFromRoot(input), kindValue);
    }
    case "npm": {
      const { flags } = parseFlags(rest);
      const input = flagString(flags, "input", "artifacts");
      const output = flagString(flags, "output", "npm");
      return distNpm(pathFromRoot(input), pathFromRoot(output));
    }
    default:
      throw new Error(`Unknown dist subcommand: ${subcommand}`);
  }
}

function pathFromRoot(p: string) {
  return path.isAbsolute(p) ? p : path.join(ROOT, p);
}

type FlagValue = string | boolean;

function parseFlags(args: string[]) {
  const flags = new Map<string, FlagValue>();
  const positionals: string[] = [];
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (!arg.startsWith("--")) {
      positionals.push(arg);
      continue;
    }
    const name = arg.slice(2);
    const next = args[i + 1];
    if (next && !next.startsWith("--")) {
      flags.set(name, next);
      i += 1;
    } else {
      flags.set(name, true);
    }
  }
  return { flags, positionals };
}

function flagString(
  flags: Map<string, FlagValue>,
  name: string,
  fallback: string,
) {
  const value = flags.get(name);
  if (value === undefined) {
    return fallback;
  }
  if (value === true) {
    throw new Error(`Missing value for --${name}`);
  }
  return value;
}

function cargoTomlPath(root: string) {
  return path.join(root, "Cargo.toml");
}

function packageJsonPath(root: string) {
  return path.join(root, "package.json");
}

function npmPlatformPackagePaths(root: string) {
  const npmRoot = path.join(root, "npm");
  if (!fs.existsSync(npmRoot)) {
    return [] as string[];
  }
  const entries = fs.readdirSync(npmRoot, { withFileTypes: true });
  const paths: string[] = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const packageJson = path.join(npmRoot, entry.name, "package.json");
    if (fs.existsSync(packageJson)) {
      paths.push(packageJson);
    }
  }
  paths.sort();
  return paths;
}

function readVersions(root: string) {
  const cargoVersion = readCargoVersion(cargoTomlPath(root));
  const packageVersion = readPackageVersion(packageJsonPath(root));
  return { cargoVersion, packageVersion };
}

function readCargoVersion(filePath: string) {
  const contents = fs.readFileSync(filePath, "utf8");
  const workspaceVersion = readTomlVersion(contents, "workspace.package");
  if (workspaceVersion) {
    return workspaceVersion;
  }
  const packageVersion = readTomlVersion(contents, "package");
  if (packageVersion) {
    return packageVersion;
  }
  throw new Error("Could not find version in Cargo.toml");
}

function writeCargoVersion(filePath: string, version: string) {
  const contents = fs.readFileSync(filePath, "utf8");
  const workspaceVersion = readTomlVersion(contents, "workspace.package");
  if (workspaceVersion) {
    const updated = updateTomlVersion(contents, "workspace.package", version);
    fs.writeFileSync(filePath, updated, "utf8");
    return;
  }
  const packageVersion = readTomlVersion(contents, "package");
  if (packageVersion) {
    const updated = updateTomlVersion(contents, "package", version);
    fs.writeFileSync(filePath, updated, "utf8");
    return;
  }
  throw new Error("Could not find version in Cargo.toml");
}

function readTomlVersion(contents: string, section: string) {
  const lines = contents.split(/\r?\n/);
  let inSection = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inSection = trimmed === `[${section}]`;
      continue;
    }
    if (!inSection) {
      continue;
    }
    const match = line.match(/^\s*version\s*=\s*"([^"]+)"/);
    if (match) {
      return match[1];
    }
  }
  return null;
}

function updateTomlVersion(contents: string, section: string, version: string) {
  const lines = contents.split(/\r?\n/);
  let inSection = false;
  let updated = false;
  for (let i = 0; i < lines.length; i += 1) {
    const trimmed = lines[i].trim();
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inSection = trimmed === `[${section}]`;
      continue;
    }
    if (!inSection) {
      continue;
    }
    if (/^\s*version\s*=/.test(lines[i])) {
      lines[i] = lines[i].replace(/version\s*=\s*"[^"]*"/, `version = "${version}"`);
      updated = true;
      break;
    }
  }
  if (!updated) {
    throw new Error(`Could not find version in [${section}]`);
  }
  const trailingNewline = contents.endsWith("\n") ? "\n" : "";
  return lines.join("\n") + trailingNewline;
}

function readPackageVersion(filePath: string) {
  const json = readPackageJson(filePath) as { version?: string };
  if (!json.version) {
    throw new Error("Could not find version in package.json");
  }
  return json.version;
}

function readPackageJson(filePath: string) {
  const contents = fs.readFileSync(filePath, "utf8");
  return JSON.parse(contents) as Record<string, unknown>;
}

function updateOptionalDependencies(
  json: Record<string, unknown>,
  version: string,
) {
  const optionalDeps = json.optionalDependencies;
  if (!optionalDeps || typeof optionalDeps !== "object") {
    return;
  }
  const deps = optionalDeps as Record<string, string>;
  for (const name of Object.keys(deps)) {
    if (name.startsWith("agent-tui-")) {
      deps[name] = version;
    }
  }
}

function writePackageVersion(filePath: string, version: string) {
  const json = readPackageJson(filePath);
  json.version = version;
  updateOptionalDependencies(json, version);
  fs.writeFileSync(filePath, `${JSON.stringify(json, null, 2)}\n`, "utf8");
}

function versionCheck(quiet: boolean) {
  const { cargoVersion, packageVersion } = readVersions(ROOT);
  const npmPaths = npmPlatformPackagePaths(ROOT);
  const packageJson = readPackageJson(packageJsonPath(ROOT));
  const optionalDeps =
    typeof packageJson.optionalDependencies === "object" &&
    packageJson.optionalDependencies
      ? (packageJson.optionalDependencies as Record<string, string>)
      : null;
  const npmPackageNames = new Set<string>();

  if (cargoVersion !== packageVersion) {
    throw new Error(
      `Version mismatch detected!\n  Cargo.toml:   ${cargoVersion}\n  package.json: ${packageVersion}`,
    );
  }

  for (const npmPath of npmPaths) {
    const npmJson = readPackageJson(npmPath);
    const npmVersion = npmJson.version;
    const npmName = npmJson.name;
    if (!npmVersion || !npmName) {
      throw new Error(`Could not find name/version in ${npmPath}`);
    }
    npmPackageNames.add(npmName);
    if (npmVersion !== packageVersion) {
      throw new Error(
        `Version mismatch detected!\n  package.json: ${packageVersion}\n  ${npmPath}: ${npmVersion}`,
      );
    }
  }

  if (npmPackageNames.size > 0 && !optionalDeps) {
    throw new Error("Missing optionalDependencies in package.json");
  }

  if (optionalDeps) {
    for (const [name, version] of Object.entries(optionalDeps)) {
      if (!name.startsWith("agent-tui-")) {
        continue;
      }
      if (version !== packageVersion) {
        throw new Error(
          `Version mismatch detected!\n  package.json: ${packageVersion}\n  optionalDependencies.${name}: ${version}`,
        );
      }
      if (!npmPackageNames.has(name)) {
        throw new Error(
          `Optional dependency ${name} has no matching npm package under npm/`,
        );
      }
    }
    for (const npmName of npmPackageNames) {
      if (!optionalDeps[npmName]) {
        throw new Error(
          `Missing optionalDependencies entry for ${npmName} in package.json`,
        );
      }
    }
  }

  if (!quiet) {
    console.log(`Version check passed: ${cargoVersion}`);
  }
}

function normalizeTag(tag: string) {
  let normalized = tag;
  if (normalized.startsWith("refs/tags/")) {
    normalized = normalized.slice("refs/tags/".length);
  }
  if (normalized.startsWith("v")) {
    normalized = normalized.slice(1);
  }
  return normalized;
}

function assertTag(tag: string) {
  const { cargoVersion, packageVersion } = readVersions(ROOT);
  const tagVersion = normalizeTag(tag);

  if (cargoVersion !== packageVersion) {
    throw new Error(
      `Version mismatch detected!\n  Cargo.toml:   ${cargoVersion}\n  package.json: ${packageVersion}`,
    );
  }

  if (cargoVersion !== tagVersion) {
    throw new Error(
      `Version mismatch between Cargo.toml (${cargoVersion}) and tag (${tag})`,
    );
  }
}

function assertInput(version: string) {
  const { cargoVersion, packageVersion } = readVersions(ROOT);

  if (cargoVersion !== packageVersion) {
    throw new Error(
      `Version mismatch detected!\n  Cargo.toml:   ${cargoVersion}\n  package.json: ${packageVersion}`,
    );
  }

  if (cargoVersion !== version) {
    throw new Error(
      `Version mismatch between Cargo.toml (${cargoVersion}) and input (${version})`,
    );
  }
}

function release(
  versionOrBump: string,
  options: { confirm: boolean },
) {
  const cargoPath = cargoTomlPath(ROOT);
  const packagePath = packageJsonPath(ROOT);
  const npmPaths = npmPlatformPackagePaths(ROOT);

  const currentVersion = readCargoVersion(cargoPath);
  const targetVersion = isBump(versionOrBump)
    ? bumpVersion(currentVersion, versionOrBump)
    : ensureSemver(versionOrBump);

  const tag = `v${targetVersion}`;

  ensureGitClean(ROOT);
  ensureTagAbsent(ROOT, tag);

  console.log(`Releasing version ${targetVersion}...`);

  console.log("Updating package.json...");
  writePackageVersion(packagePath, targetVersion);
  for (const npmPath of npmPaths) {
    writePackageVersion(npmPath, targetVersion);
  }

  console.log("Updating Cargo.toml...");
  writeCargoVersion(cargoPath, targetVersion);

  const status = gitStatusShort(ROOT);
  if (options.confirm) {
    console.log("Changes to be released:");
    console.log(status ? status : "(no changes detected)");
    if (!confirmProceed("Stage, commit, and tag these changes?")) {
      console.log("Release aborted before staging.");
      return;
    }
  }

  console.log("Staging changes...");
  runCommand("git", ["add", "-A"], { cwd: ROOT });

  console.log("Committing...");
  runCommand(
    "git",
    ["commit", "-m", `chore: bump version to ${targetVersion}`],
    { cwd: ROOT },
  );

  console.log(`Creating tag ${tag}...`);
  runCommand("git", ["tag", "-a", tag, "-m", `Release ${targetVersion}`], {
    cwd: ROOT,
  });

  console.log(`Done! Release ${targetVersion} prepared.`);
  console.log("Nothing has been pushed yet. To publish when ready, run:");
  console.log("  git push && git push --tags");
}

function isBump(value: string) {
  return value === "major" || value === "minor" || value === "patch";
}

function ensureSemver(value: string) {
  parseSemver(value);
  return value;
}

function parseSemver(value: string) {
  const match = value.match(/^(\d+)\.(\d+)\.(\d+)(?:[-+].+)?$/);
  if (!match) {
    throw new Error(`Invalid version: ${value}`);
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
  };
}

function bumpVersion(current: string, bump: string) {
  const version = parseSemver(current);
  switch (bump) {
    case "major":
      return `${version.major + 1}.0.0`;
    case "minor":
      return `${version.major}.${version.minor + 1}.0`;
    case "patch":
      return `${version.major}.${version.minor}.${version.patch + 1}`;
    default:
      return current;
  }
}

function ensureGitClean(root: string) {
  const status = spawnSync("git", ["status", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
  });
  if (status.status !== 0) {
    throw new Error("Command failed: git status --porcelain");
  }
  if (status.stdout.trim().length > 0) {
    throw new Error(
      "You have uncommitted or untracked changes. Please commit or stash them first.",
    );
  }
}

function resolveConfirm(flags: Map<string, FlagValue>) {
  const value = flags.get("yes");
  if (value === undefined) {
    return true;
  }
  if (value === true) {
    return false;
  }
  throw new Error("Invalid value for --yes");
}

function gitStatusShort(root: string) {
  const status = spawnSync("git", ["status", "--short"], {
    cwd: root,
    encoding: "utf8",
  });
  if (status.status !== 0) {
    throw new Error("Command failed: git status --short");
  }
  return status.stdout.trimEnd();
}

function confirmProceed(message: string) {
  if (!process.stdin.isTTY) {
    throw new Error(
      "Confirmation required but stdin is not a TTY. Re-run with --yes to skip confirmation.",
    );
  }
  process.stdout.write(`${message} [y/N] `);
  const input = readStdinLine();
  const answer = input.trim().toLowerCase();
  return answer === "y" || answer === "yes";
}

function readStdinLine() {
  const buffer = Buffer.alloc(1024);
  let input = "";
  while (true) {
    const bytes = fs.readSync(0, buffer, 0, buffer.length, null);
    if (bytes === 0) {
      break;
    }
    input += buffer.toString("utf8", 0, bytes);
    if (input.includes("\n")) {
      break;
    }
  }
  return input;
}

function ensureTagAbsent(root: string, tag: string) {
  const result = spawnSync("git", ["rev-parse", tag], {
    cwd: root,
    stdio: "ignore",
  });
  if (result.status === 0) {
    throw new Error(`Tag ${tag} already exists`);
  }
}


function ci() {
  console.log("Running CI checks...");

  runStep("Checking formatting", () => {
    runCommand("cargo", ["fmt", "--all", "--", "--check"], { cwd: ROOT });
  });

  runStep("Running clippy", () => {
    runCommand(
      "cargo",
      ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
      { cwd: ROOT },
    );
  });

  if (hasCommand("ast-grep")) {
    runStep("Running ast-grep Clean Architecture checks", () => {
      runCommand("ast-grep", ["scan", "--config", "sgconfig.yml"], {
        cwd: ROOT,
      });
    });
  } else {
    console.log("ast-grep not installed, skipping...");
  }

  runStep("Running architecture checks", () => architectureCheck());

  runStep("Running tests", () => {
    runCommand("cargo", ["test", "--workspace"], { cwd: ROOT });
  });

  if (hasCommand("cargo-machete")) {
    runStep("Checking for unused dependencies", () => {
      try {
        runCommand("cargo-machete", ["--skip-target-dir", "."], {
          cwd: ROOT,
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        console.log(`cargo-machete failed, skipping: ${message}`);
      }
    });
  } else {
    console.log("cargo-machete not installed, skipping...");
  }

  runStep("Checking version consistency", () => versionCheck(true));

  console.log("All checks passed!");
}

function runStep(label: string, action: () => void) {
  console.log(`\n-> ${label}...`);
  action();
}

function runCommand(
  program: string,
  args: string[],
  options: { cwd?: string } = {},
) {
  const result = spawnSync(program, args, {
    cwd: options.cwd,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${program} ${args.join(" ")}`);
  }
}

function hasCommand(name: string) {
  const envPath = process.env.PATH ?? "";
  const paths = envPath.split(path.delimiter).filter(Boolean);
  const isWindows = process.platform === "win32";
  const exts = isWindows
    ? (process.env.PATHEXT ?? ".EXE;.CMD;.BAT")
        .split(";")
        .filter(Boolean)
    : [""];

  for (const dir of paths) {
    for (const ext of exts) {
      const candidate = path.join(dir, `${name}${ext}`);
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
        return true;
      }
    }
  }

  return false;
}


const LEGACY_SHIM_REGEX =
  /crate::daemon::|crate::ipc::|crate::terminal::|crate::core::|crate::commands::|crate::handlers::|crate::presenter::|crate::error::|crate::attach::/;

function architectureCheck() {
  const srcRoot = path.join(ROOT, "crates", "agent-tui", "src");

  const legacyShim = findFirstMatch(srcRoot, (_path, line) =>
    LEGACY_SHIM_REGEX.test(line),
  );
  if (legacyShim) {
    throw new Error(
      `Architecture check failed: legacy shim paths detected at ${legacyShim}`,
    );
  }

  const exitUsage = findFirstMatch(srcRoot, (filePath, line) => {
    return line.includes("std::process::exit") && !filePath.endsWith("main.rs");
  });
  if (exitUsage) {
    throw new Error(
      `Architecture check failed: std::process::exit is only allowed in main.rs (found at ${exitUsage})`,
    );
  }

  const domainRoot = path.join(srcRoot, "domain");
  const domainSerde = findFirstMatch(domainRoot, (_path, line) =>
    line.includes("serde_json"),
  );
  if (domainSerde) {
    throw new Error(
      `Architecture check failed: serde_json usage detected in domain (found at ${domainSerde})`,
    );
  }

  const usecasesRoot = path.join(srcRoot, "usecases");
  const usecasesSerde = findFirstMatch(usecasesRoot, (_path, line) =>
    line.includes("serde_json"),
  );
  if (usecasesSerde) {
    throw new Error(
      `Architecture check failed: serde_json usage detected in usecases (found at ${usecasesSerde})`,
    );
  }

  const infraDeps = findFirstMatch(usecasesRoot, (filePath, line) => {
    if (filePath.endsWith(path.join("ports", "errors.rs"))) {
      return false;
    }
    if (!line.includes("crate::infra::")) {
      return false;
    }
    return !line.includes("crate::infra::daemon::test_support");
  });
  if (infraDeps) {
    throw new Error(
      `Architecture check failed: infra dependency detected in usecases (found at ${infraDeps})`,
    );
  }

  console.log("Architecture checks passed.");
}

function findFirstMatch(
  root: string,
  matcher: (filePath: string, line: string) => boolean,
) {
  if (!fs.existsSync(root)) {
    return null;
  }

  const files = walkDir(root);
  for (const filePath of files) {
    if (path.extname(filePath) !== ".rs") {
      continue;
    }

    let contents: string;
    try {
      contents = fs.readFileSync(filePath, "utf8");
    } catch {
      continue;
    }

    const lines = contents.split(/\r?\n/);
    for (let index = 0; index < lines.length; index += 1) {
      if (matcher(filePath, lines[index])) {
        const relative = path.relative(root, filePath);
        return `${relative}:${index + 1}`;
      }
    }
  }

  return null;
}

function walkDir(dir: string) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkDir(fullPath));
    } else if (entry.isFile()) {
      files.push(fullPath);
    }
  }
  return files;
}

type DistKind = "release" | "npm";

function distVerify(input: string, kind: DistKind) {
  if (!fs.existsSync(input)) {
    throw new Error(`Artifacts directory not found: ${input}`);
  }

  const missing: string[] = [];
  for (const name of requiredArtifacts(kind)) {
    const artifact = artifactPath(input, name);
    if (!fs.existsSync(artifact)) {
      missing.push(artifact);
    }
  }

  if (missing.length > 0) {
    throw new Error(`Missing required artifacts:\n${missing.join("\n")}`);
  }

  console.log(`All required artifacts present for ${kind}.`);
}

function distRelease(input: string, output: string) {
  distVerify(input, "release");

  if (!fs.existsSync(input)) {
    throw new Error(`Artifacts directory not found: ${input}`);
  }

  fs.mkdirSync(output, { recursive: true });

  const seen = new Set<string>();
  const files = walkDir(input).filter((filePath) =>
    fs.statSync(filePath).isFile(),
  );
  for (const filePath of files) {
    const fileName = path.basename(filePath);
    if (seen.has(fileName)) {
      throw new Error(`Duplicate artifact filename: ${fileName}`);
    }
    seen.add(fileName);

    const dest = path.join(output, fileName);
    fs.copyFileSync(filePath, dest);
    makeExecutable(dest);
  }

  const outputFiles = fs
    .readdirSync(output, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .filter((name) => name !== "checksums-sha256.txt")
    .sort();

  let checksums = "";
  for (const fileName of outputFiles) {
    const filePath = path.join(output, fileName);
    const digest = sha256File(filePath);
    checksums += `${digest}  ${fileName}\n`;
  }

  fs.writeFileSync(path.join(output, "checksums-sha256.txt"), checksums, "utf8");
  console.log(`Prepared release assets in ${output}`);
}

function distNpm(input: string, output: string) {
  if (!fs.existsSync(input)) {
    throw new Error(`Artifacts directory not found: ${input}`);
  }

  for (const name of requiredArtifacts("npm")) {
    const artifact = artifactPath(input, name);
    if (!fs.existsSync(artifact)) {
      throw new Error(`Missing artifact: ${artifact}`);
    }
    const packageDir = path.join(output, name);
    const packageJson = path.join(packageDir, "package.json");
    if (!fs.existsSync(packageJson)) {
      throw new Error(`Missing npm package: ${packageJson}`);
    }

    const binDir = path.join(packageDir, "bin");
    fs.mkdirSync(binDir, { recursive: true });
    const dest = path.join(binDir, "agent-tui");
    fs.copyFileSync(artifact, dest);
    makeExecutable(dest);
  }

  console.log(`Prepared npm platform packages in ${output}`);
}

function sha256File(filePath: string) {
  const data = fs.readFileSync(filePath);
  return crypto.createHash("sha256").update(data).digest("hex");
}

function makeExecutable(filePath: string) {
  if (process.platform === "win32") {
    return;
  }
  fs.chmodSync(filePath, 0o755);
}

function requiredArtifacts(kind: DistKind) {
  if (kind === "release") {
    return [
      "agent-tui-linux-x64",
      "agent-tui-linux-arm64",
      "agent-tui-linux-x64-musl",
      "agent-tui-linux-arm64-musl",
      "agent-tui-darwin-x64",
      "agent-tui-darwin-arm64",
    ];
  }
  return [
    "agent-tui-linux-x64",
    "agent-tui-linux-arm64",
    "agent-tui-darwin-x64",
    "agent-tui-darwin-arm64",
  ];
}

function artifactPath(input: string, name: string) {
  return path.join(input, name, name);
}

main();
