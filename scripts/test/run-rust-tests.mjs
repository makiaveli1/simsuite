import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import { fileURLToPath } from "node:url";
import path from "node:path";
import {
  isWslEnvironment,
  resolvePowerShellExecutable,
  toWindowsPath,
} from "../desktop/run-powershell-script.mjs";

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

function quotePowerShell(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

function quotePowerShellArgument(value) {
  if (/^[A-Za-z0-9_:./\\=-]+$/.test(value)) {
    return value;
  }

  return quotePowerShell(value);
}

export function buildRustTestInvocation({
  cwd = process.cwd(),
  argv = process.argv,
  env = process.env,
  platform = process.platform,
  releaseText = readLinuxReleaseText(platform),
} = {}) {
  const passthroughArgs = argv.slice(2);
  const cargoArgs = ["test", "--manifest-path", "src-tauri/Cargo.toml", ...passthroughArgs];
  const wsl = isWslEnvironment({ env, platform, releaseText });

  if (platform === "win32" || wsl) {
    const command = resolvePowerShellExecutable({ env, platform, releaseText });
    const windowsCwd = platform === "win32" ? cwd : toWindowsPath(cwd);
    const cargoCommand = ["cargo", ...cargoArgs].map(quotePowerShellArgument).join(" ");
    const script = [
      "$ErrorActionPreference = 'Stop'",
      `Set-Location -LiteralPath ${quotePowerShell(windowsCwd)}`,
      cargoCommand,
      "exit $LASTEXITCODE",
    ].join("; ");

    return {
      command,
      args: ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    };
  }

  return {
    command: "cargo",
    args: cargoArgs,
  };
}

export function run(argv = process.argv) {
  const invocation = buildRustTestInvocation({ argv });
  const result = spawnSync(invocation.command, invocation.args, {
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    throw new Error(`Could not start ${invocation.command}: ${result.error.message}`);
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
