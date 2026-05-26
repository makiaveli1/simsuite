#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

function executableForNpx(platform = process.platform) {
  return platform === "win32" ? "npx.cmd" : "npx";
}

export function buildVitestInvocation({
  argv = process.argv,
  env = process.env,
  platform = process.platform,
} = {}) {
  const passthroughArgs = argv.slice(2);

  return {
    command: executableForNpx(platform),
    args: ["vitest", "run", ...passthroughArgs],
    env: {
      ...env,
      NODE_ENV: "test",
    },
  };
}

export function run(argv = process.argv) {
  const invocation = buildVitestInvocation({ argv });
  const result = spawnSync(invocation.command, invocation.args, {
    env: invocation.env,
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
