#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import {
  buildPowerShellInvocation,
  isWslEnvironment,
} from "../desktop/run-powershell-script.mjs";

export const DEFAULT_DEV_PORT = 1420;

function readLinuxReleaseText(platform = process.platform) {
  if (platform !== "linux") {
    return os.release();
  }

  try {
    return fs.readFileSync("/proc/version", "utf8");
  } catch {
    return os.release();
  }
}

export function assertSimSuiteRepo(cwd = process.cwd()) {
  const packagePath = path.join(cwd, "package.json");
  if (!fs.existsSync(packagePath)) {
    throw new Error(`No package.json found in ${cwd}. Run this command from the SimSuite repo root.`);
  }

  const packageJson = JSON.parse(fs.readFileSync(packagePath, "utf8"));
  if (packageJson.name !== "simsuite") {
    throw new Error(`Expected the SimSuite repo, found package "${packageJson.name ?? "unknown"}".`);
  }
}

export function isWindowsDesktopLane({
  env = process.env,
  platform = process.platform,
  releaseText = readLinuxReleaseText(platform),
} = {}) {
  return platform === "win32" || isWslEnvironment({ env, platform, releaseText });
}

export function normalizeTauriEnvironment(env = process.env) {
  const normalized = { ...env };
  if (normalized.CI !== undefined) {
    const value = String(normalized.CI).trim().toLowerCase();
    normalized.CI = ["", "0", "false", "no", "off"].includes(value) ? "false" : "true";
  }
  return normalized;
}

function localTauriCliPath(cwd, platform = process.platform) {
  const pathApi = platform === "win32" ? path.win32 : path.posix;
  return pathApi.join(cwd, "node_modules", "@tauri-apps", "cli", "tauri.js");
}

export function buildPortPreparationInvocation({
  cwd = process.cwd(),
  env = process.env,
  platform = process.platform,
  releaseText = readLinuxReleaseText(platform),
  port = DEFAULT_DEV_PORT,
  quiet = false,
} = {}) {
  if (!isWindowsDesktopLane({ env, platform, releaseText })) {
    return null;
  }

  const scriptArgs = ["-Port", String(port)];
  if (quiet) {
    scriptArgs.push("-Quiet");
  }

  const invocation = buildPowerShellInvocation({
    cwd,
    env,
    platform,
    releaseText,
    script: "scripts/dev/cleanup-dev-port.ps1",
    scriptArgs,
  });

  return {
    command: invocation.executable,
    args: invocation.args,
    cwd,
  };
}

export function buildTauriDevInvocation({
  cwd = process.cwd(),
  env = process.env,
  platform = process.platform,
  releaseText = readLinuxReleaseText(platform),
  port = DEFAULT_DEV_PORT,
  passthroughArgs = [],
} = {}) {
  if (isWindowsDesktopLane({ env, platform, releaseText })) {
    const invocation = buildPowerShellInvocation({
      cwd,
      env,
      platform,
      releaseText,
      script: "scripts/dev/run-tauri-dev.ps1",
      scriptArgs: ["-Port", String(port), ...passthroughArgs],
    });

    return {
      command: invocation.executable,
      args: invocation.args,
      cwd,
      env: normalizeTauriEnvironment(env),
      preparesPortInternally: true,
    };
  }

  const cliPath = localTauriCliPath(cwd, platform);
  return {
    command: process.execPath,
    args: [cliPath, "dev", ...passthroughArgs],
    cwd,
    env: normalizeTauriEnvironment(env),
    requiredPath: cliPath,
    preparesPortInternally: false,
  };
}

export function buildTauriBuildInvocation({
  cwd = process.cwd(),
  env = process.env,
  platform = process.platform,
  passthroughArgs = [],
} = {}) {
  const cliPath = localTauriCliPath(cwd, platform);
  return {
    command: process.execPath,
    args: [cliPath, "build", ...passthroughArgs],
    cwd,
    env: normalizeTauriEnvironment(env),
    requiredPath: cliPath,
  };
}

async function assertHostPortAvailable(port, host, ignoreUnsupportedAddress) {
  await new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.once("error", (error) => {
      if (error && error.code === "EADDRINUSE") {
        reject(
          new Error(
            `Port ${port} is already in use. SimSuite did not stop an unknown process automatically. Close the existing dev server and try again.`,
          ),
        );
        return;
      }
      if (
        ignoreUnsupportedAddress &&
        error &&
        ["EAFNOSUPPORT", "EADDRNOTAVAIL"].includes(error.code)
      ) {
        resolve();
        return;
      }
      reject(error);
    });
    server.listen({ port, host, exclusive: true }, () => {
      server.close((error) => {
        if (error) {
          reject(error);
        } else {
          resolve();
        }
      });
    });
  });
}

export async function assertPortAvailable(port = DEFAULT_DEV_PORT) {
  await assertHostPortAvailable(port, "127.0.0.1", false);
  await assertHostPortAvailable(port, "::1", true);
}

function runSyncInvocation(invocation) {
  const result = spawnSync(invocation.command, invocation.args, {
    cwd: invocation.cwd,
    env: invocation.env ?? process.env,
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    throw new Error(`Could not start ${invocation.command}: ${result.error.message}`);
  }
  if ((result.status ?? 1) !== 0) {
    throw new Error(`${invocation.command} exited with status ${result.status ?? 1}.`);
  }
}

export async function prepareDevPort(options = {}) {
  const cwd = options.cwd ?? process.cwd();
  const port = options.port ?? DEFAULT_DEV_PORT;
  assertSimSuiteRepo(cwd);

  const invocation = buildPortPreparationInvocation({ ...options, cwd, port });
  if (invocation) {
    runSyncInvocation(invocation);
    return;
  }

  await assertPortAvailable(port);
  if (!options.quiet) {
    console.log(`SIMSUITE_DEV_PORT status=free port=${port}`);
  }
}

export async function runTauriDev(options = {}) {
  const cwd = options.cwd ?? process.cwd();
  const port = options.port ?? DEFAULT_DEV_PORT;
  assertSimSuiteRepo(cwd);

  const invocation = buildTauriDevInvocation({ ...options, cwd, port });
  if (!invocation.preparesPortInternally) {
    await assertPortAvailable(port);
    if (invocation.requiredPath && !fs.existsSync(invocation.requiredPath)) {
      throw new Error(
        `SimSuite could not find the local Tauri CLI at ${invocation.requiredPath}. Run pnpm install first.`,
      );
    }
  }

  return await new Promise((resolve, reject) => {
    const child = spawn(invocation.command, invocation.args, {
      cwd: invocation.cwd,
      env: invocation.env ?? process.env,
      stdio: "inherit",
      shell: false,
    });

    const signalHandlers = new Map();
    for (const signal of ["SIGINT", "SIGTERM"]) {
      const handler = () => child.kill(signal);
      signalHandlers.set(signal, handler);
      process.once(signal, handler);
    }

    const clearHandlers = () => {
      for (const [signal, handler] of signalHandlers) {
        process.removeListener(signal, handler);
      }
    };

    child.once("error", (error) => {
      clearHandlers();
      reject(new Error(`Could not start ${invocation.command}: ${error.message}`));
    });
    child.once("exit", (code, signal) => {
      clearHandlers();
      if (signal) {
        resolve(1);
      } else {
        resolve(code ?? 1);
      }
    });
  });
}


export function runTauriBuild(options = {}) {
  const cwd = options.cwd ?? process.cwd();
  assertSimSuiteRepo(cwd);

  const invocation = buildTauriBuildInvocation({ ...options, cwd });
  if (invocation.requiredPath && !fs.existsSync(invocation.requiredPath)) {
    throw new Error(
      `SimSuite could not find the local Tauri CLI at ${invocation.requiredPath}. Run pnpm install first.`,
    );
  }

  runSyncInvocation(invocation);
  return 0;
}
