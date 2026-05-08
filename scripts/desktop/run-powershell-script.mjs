#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const WSL_SYSTEM_POWERSHELL = "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe";

function looksWindowsPath(value) {
  return /^[A-Za-z]:[\\/]/.test(value);
}

function splitScriptPath(value) {
  return value.split(/[\\/]+/).filter(Boolean);
}

export function isWslEnvironment({
  env = process.env,
  platform = process.platform,
  releaseText = os.release(),
} = {}) {
  if (platform !== "linux") {
    return false;
  }

  return Boolean(
    env.WSL_DISTRO_NAME ||
      env.WSL_INTEROP ||
      /microsoft|wsl/i.test(releaseText),
  );
}

export function toWindowsPath(value) {
  if (!value || typeof value !== "string") {
    throw new Error("Cannot convert an empty path to a Windows path.");
  }

  if (looksWindowsPath(value)) {
    return value.replace(/\//g, "\\");
  }

  const wslMount = value.match(/^\/mnt\/([A-Za-z])(?:\/(.*))?$/);
  if (wslMount) {
    const drive = wslMount[1].toUpperCase();
    const rest = (wslMount[2] ?? "").replace(/\//g, "\\");
    return rest ? `${drive}:\\${rest}` : `${drive}:\\`;
  }

  const gitBashMount = value.match(/^\/([A-Za-z])\/(.+)$/);
  if (gitBashMount) {
    return `${gitBashMount[1].toUpperCase()}:\\${gitBashMount[2].replace(/\//g, "\\")}`;
  }

  throw new Error(`Cannot convert "${value}" to a Windows path.`);
}

export function resolveScriptPath(cwd, script) {
  if (!script) {
    throw new Error("Missing PowerShell script path.");
  }

  if (looksWindowsPath(script)) {
    return path.win32.normalize(script);
  }

  if (script.startsWith("/")) {
    return path.posix.normalize(script);
  }

  if (looksWindowsPath(cwd)) {
    return path.win32.join(cwd, ...splitScriptPath(script));
  }

  return path.posix.join(cwd, ...splitScriptPath(script));
}

export function resolvePowerShellExecutable({
  env = process.env,
  platform = process.platform,
  releaseText = os.release(),
} = {}) {
  if (env.SIMSUITE_POWERSHELL_PATH) {
    return env.SIMSUITE_POWERSHELL_PATH;
  }

  if (platform === "win32") {
    return "powershell.exe";
  }

  if (isWslEnvironment({ env, platform, releaseText })) {
    return WSL_SYSTEM_POWERSHELL;
  }

  throw new Error(
    "Desktop verification requires Windows PowerShell from Windows or WSL.",
  );
}

export function buildPowerShellInvocation({
  cwd = process.cwd(),
  env = process.env,
  platform = process.platform,
  releaseText = os.release(),
  script,
  scriptArgs = [],
}) {
  const scriptPath = resolveScriptPath(cwd, script);
  const wsl = isWslEnvironment({ env, platform, releaseText });
  const executable = resolvePowerShellExecutable({ env, platform, releaseText });

  if (platform !== "win32" && !wsl) {
    throw new Error(
      "Desktop verification wrappers can only run from native Windows or WSL.",
    );
  }

  const scriptFile = platform === "win32" || wsl ? toWindowsPath(scriptPath) : scriptPath;

  return {
    executable,
    args: [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      scriptFile,
      ...scriptArgs,
    ],
    scriptPath,
  };
}

function assertRepoRoot(cwd) {
  const packagePath = path.join(cwd, "package.json");
  if (!fs.existsSync(packagePath)) {
    throw new Error(`No package.json found in ${cwd}. Run this command from the repo root.`);
  }

  const packageJson = JSON.parse(fs.readFileSync(packagePath, "utf8"));
  if (packageJson.name !== "simsuite") {
    throw new Error(`Expected SimSuite repo root, found package "${packageJson.name ?? "unknown"}".`);
  }
}

function readLinuxReleaseText(platform) {
  if (platform !== "linux") {
    return os.release();
  }

  try {
    return fs.readFileSync("/proc/version", "utf8");
  } catch {
    return os.release();
  }
}

export function run(argv = process.argv) {
  const [, , script, ...scriptArgs] = argv;
  const cwd = process.cwd();
  assertRepoRoot(cwd);

  if (!script) {
    throw new Error(
      "Usage: node scripts/desktop/run-powershell-script.mjs <script.ps1> [args...]",
    );
  }

  const releaseText = readLinuxReleaseText(process.platform);
  const invocation = buildPowerShellInvocation({
    cwd,
    env: process.env,
    platform: process.platform,
    releaseText,
    script,
    scriptArgs,
  });

  if (!fs.existsSync(invocation.scriptPath)) {
    throw new Error(`PowerShell script not found: ${invocation.scriptPath}`);
  }

  const result = spawnSync(invocation.executable, invocation.args, {
    cwd,
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    throw new Error(
      `Could not start PowerShell executable "${invocation.executable}": ${result.error.message}`,
    );
  }

  return result.status ?? 1;
}

function main() {
  try {
    process.exitCode = run();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && path.resolve(process.argv[1]) === modulePath) {
  main();
}
