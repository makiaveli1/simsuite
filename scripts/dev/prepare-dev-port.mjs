#!/usr/bin/env node

import { fileURLToPath } from "node:url";
import path from "node:path";
import { DEFAULT_DEV_PORT, prepareDevPort } from "./dev-runtime.mjs";

function parsePort(argv) {
  const portIndex = argv.findIndex((value) => value === "--port");
  const inlinePort = argv.find((value) => value.startsWith("--port="));
  const rawPort = inlinePort?.slice("--port=".length) ??
    (portIndex >= 0 ? argv[portIndex + 1] : undefined);
  const port = rawPort === undefined ? DEFAULT_DEV_PORT : Number.parseInt(rawPort, 10);

  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`Invalid development port: ${rawPort ?? ""}`);
  }
  return port;
}

export async function run(argv = process.argv.slice(2)) {
  await prepareDevPort({
    port: parsePort(argv),
    quiet: argv.includes("--quiet"),
  });
}

async function main() {
  try {
    await run();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && path.resolve(process.argv[1]) === modulePath) {
  await main();
}
