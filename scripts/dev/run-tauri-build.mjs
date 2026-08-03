#!/usr/bin/env node

import { fileURLToPath } from "node:url";
import path from "node:path";
import { runTauriBuild } from "./dev-runtime.mjs";

export function run(argv = process.argv.slice(2)) {
  return runTauriBuild({ passthroughArgs: argv });
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
