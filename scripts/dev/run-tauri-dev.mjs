#!/usr/bin/env node

import { fileURLToPath } from "node:url";
import path from "node:path";
import { DEFAULT_DEV_PORT, runTauriDev } from "./dev-runtime.mjs";

function parseArguments(argv) {
  const passthroughArgs = [];
  let port = DEFAULT_DEV_PORT;

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === "--port") {
      port = Number.parseInt(argv[index + 1] ?? "", 10);
      index += 1;
      continue;
    }
    if (value.startsWith("--port=")) {
      port = Number.parseInt(value.slice("--port=".length), 10);
      continue;
    }
    passthroughArgs.push(value);
  }

  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`Invalid development port: ${port}`);
  }

  return { port, passthroughArgs };
}

export async function run(argv = process.argv.slice(2)) {
  const options = parseArguments(argv);
  return await runTauriDev(options);
}

async function main() {
  try {
    process.exitCode = await run();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && path.resolve(process.argv[1]) === modulePath) {
  await main();
}
